//! mDNS discovery methods for [`ConnectionManager`].

use std::time::Duration;

use tracing::warn;

use capydeploy_discovery::client::Client as DiscoveryClient;
use capydeploy_discovery::types::EventType;

use crate::manager::ConnectionManager;
use crate::reconnection::cancel_reconnect_for;
use crate::types::{ConnectionEvent, ConnectionState};

impl ConnectionManager {
    /// Starts continuous mDNS discovery in the background.
    pub async fn start_discovery(&self, interval: Duration) {
        let cancel_rx = self.cancel_rx.clone();

        // Take events before spawning.
        let events_rx = self.discovery.lock().await.take_events();

        // Start the mDNS browsing loop.
        let discovery = self.discovery.clone();
        tokio::spawn(async move {
            let disc = discovery.lock().await;
            disc.start_continuous_discovery(cancel_rx, interval).await;
        });

        // Start the event forwarding loop.
        if let Some(mut rx) = events_rx {
            let discovered = self.discovered.clone();
            let state = self.state.clone();
            let events_tx = self.events_tx.clone();
            let cancel_rx = self.cancel_rx.clone();
            let reconnect_cancel = self.reconnect_cancel.clone();

            tokio::spawn(async move {
                let mut cancel = cancel_rx;
                loop {
                    tokio::select! {
                        _ = cancel.changed() => break,
                        event = rx.recv() => {
                            match event {
                                Some(e) => {
                                    match e.event_type {
                                        EventType::Discovered => {
                                            let id = e.agent.info.id.clone();
                                            discovered.write().await.insert(id.clone(), e.agent.clone());
                                            state.write().await.insert(id, ConnectionState::Discovered);
                                            let _ = events_tx.send(ConnectionEvent::AgentFound(e.agent)).await;
                                        }
                                        EventType::Updated => {
                                            let id = e.agent.info.id.clone();
                                            discovered.write().await.insert(id, e.agent.clone());
                                            let _ = events_tx.send(ConnectionEvent::AgentUpdated(e.agent)).await;
                                        }
                                        EventType::Lost => {
                                            let id = e.agent.info.id.clone();
                                            discovered.write().await.remove(&id);
                                            // Don't kill reconnect or remove state while reconnecting —
                                            // the reconnect loop uses last_known_address as fallback.
                                            let is_reconnecting = state.read().await
                                                .get(&id)
                                                .is_some_and(|s| matches!(s, ConnectionState::Reconnecting { .. }));
                                            if !is_reconnecting {
                                                state.write().await.remove(&id);
                                                cancel_reconnect_for(&reconnect_cancel, &id);
                                            }
                                            let _ = events_tx.send(ConnectionEvent::AgentLost(id)).await;
                                        }
                                    }
                                }
                                None => break,
                            }
                        }
                    }
                }
            });
        }
    }

    /// Forces a fresh mDNS query to re-discover agents.
    ///
    /// Creates a temporary one-shot daemon, probes the network for a few
    /// seconds, and merges the results into the tracked set. Any new or
    /// updated agents are emitted through the existing events channel.
    pub async fn refresh_discovery(&self) {
        let client = DiscoveryClient::new();
        match client.discover(Duration::from_secs(3)).await {
            Ok(agents) => {
                for agent in agents {
                    let id = agent.info.id.clone();
                    let is_new = !self.discovered.read().await.contains_key(&id);
                    self.discovered
                        .write()
                        .await
                        .insert(id.clone(), agent.clone());

                    if is_new {
                        self.state
                            .write()
                            .await
                            .insert(id, ConnectionState::Discovered);
                        let _ = self
                            .events_tx
                            .send(ConnectionEvent::AgentFound(agent))
                            .await;
                    } else {
                        let _ = self
                            .events_tx
                            .send(ConnectionEvent::AgentUpdated(agent))
                            .await;
                    }
                }
            }
            Err(e) => {
                warn!("refresh discovery failed: {e}");
            }
        }
    }
}
