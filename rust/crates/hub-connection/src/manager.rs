//! Connection manager orchestrating discovery and WebSocket connections.
//!
//! Auto-discovers Agents via mDNS, manages WebSocket client lifecycles,
//! and tracks connection state.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, RwLock, mpsc, watch};
use tracing::{debug, info};

use capydeploy_discovery::client::Client as DiscoveryClient;
use capydeploy_discovery::types::{DiscoveredAgent, EventType};
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages::{AgentStatusResponse, HubConnectedRequest, InfoResponse};

use crate::pairing::TokenStore;
use crate::ws_client::{HandshakeResult, WsClient, WsError};

/// Connection state for an Agent.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Agent discovered via mDNS but not connected.
    Discovered,
    /// WebSocket handshake in progress.
    Connecting,
    /// Connected and authenticated.
    Connected,
    /// Agent requires pairing before connection.
    PairingRequired,
    /// Connection lost.
    Disconnected,
}

/// An Agent that has been connected.
#[derive(Debug, Clone)]
pub struct ConnectedAgent {
    pub agent: DiscoveredAgent,
    pub status: AgentStatusResponse,
}

/// Events emitted by the connection manager.
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// An Agent was discovered via mDNS.
    AgentFound(DiscoveredAgent),
    /// A discovered Agent's info was updated.
    AgentUpdated(DiscoveredAgent),
    /// A discovered Agent was lost (mDNS timeout).
    AgentLost(String),
    /// Connection state changed for an Agent.
    StateChanged {
        agent_id: String,
        state: ConnectionState,
    },
    /// Pairing is required. The UI should prompt the user for the code.
    PairingNeeded {
        agent_id: String,
        code: String,
        expires_in: i32,
    },
    /// A push event was received from a connected Agent.
    AgentEvent {
        agent_id: String,
        msg_type: MessageType,
        message: Message,
    },
}

/// Hub identity used in connection handshakes.
#[derive(Debug, Clone)]
pub struct HubIdentity {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub hub_id: String,
}

/// Connection manager for discovering and connecting to Agents.
pub struct ConnectionManager {
    hub: HubIdentity,
    discovery: Arc<Mutex<DiscoveryClient>>,
    token_store: Option<Arc<TokenStore>>,
    discovered: Arc<RwLock<HashMap<String, DiscoveredAgent>>>,
    connected: Arc<RwLock<Option<(String, ConnectedAgent)>>>,
    ws_client: Arc<Mutex<Option<WsClient>>>,
    /// Agent ID for a connection in pairing state.
    pairing_agent_id: Arc<Mutex<Option<String>>>,
    events_tx: mpsc::Sender<ConnectionEvent>,
    events_rx: Mutex<Option<mpsc::Receiver<ConnectionEvent>>>,
    cancel_tx: watch::Sender<bool>,
    cancel_rx: watch::Receiver<bool>,
    state: Arc<RwLock<HashMap<String, ConnectionState>>>,
}

impl ConnectionManager {
    /// Creates a new connection manager.
    pub fn new(hub: HubIdentity, token_store: Option<Arc<TokenStore>>) -> Self {
        let (events_tx, events_rx) = mpsc::channel(64);
        let (cancel_tx, cancel_rx) = watch::channel(false);

        Self {
            hub,
            discovery: Arc::new(Mutex::new(DiscoveryClient::new())),
            token_store,
            discovered: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(None)),
            ws_client: Arc::new(Mutex::new(None)),
            pairing_agent_id: Arc::new(Mutex::new(None)),
            events_tx,
            events_rx: Mutex::new(Some(events_rx)),
            cancel_tx,
            cancel_rx,
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Takes the event receiver. Can only be called once.
    pub async fn take_events(&self) -> Option<mpsc::Receiver<ConnectionEvent>> {
        self.events_rx.lock().await.take()
    }

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
                                            state.write().await.remove(&id);
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

    /// Returns all discovered Agents.
    pub async fn get_discovered(&self) -> Vec<DiscoveredAgent> {
        self.discovered.read().await.values().cloned().collect()
    }

    /// Returns the currently connected Agent, if any.
    pub async fn get_connected(&self) -> Option<ConnectedAgent> {
        self.connected.read().await.as_ref().map(|(_, a)| a.clone())
    }

    /// Returns the connection state for an Agent.
    pub async fn get_state(&self, agent_id: &str) -> Option<ConnectionState> {
        self.state.read().await.get(agent_id).cloned()
    }

    /// Connects to an Agent by ID.
    ///
    /// If the Agent requires pairing, `ConnectionEvent::PairingNeeded` is
    /// emitted and the WsClient is kept alive for [`confirm_pairing`](Self::confirm_pairing).
    pub async fn connect_agent(&self, agent_id: &str) -> Result<ConnectedAgent, WsError> {
        // Find the discovered agent.
        let agent = self
            .discovered
            .read()
            .await
            .get(agent_id)
            .cloned()
            .ok_or(WsError::Closed)?;

        // Disconnect existing connection.
        self.disconnect_agent().await;

        self.set_state(agent_id, ConnectionState::Connecting).await;

        let ws_url = agent.websocket_address();
        let token = self
            .token_store
            .as_ref()
            .and_then(|s| s.get_token(agent_id))
            .unwrap_or_default();

        let hub_req = HubConnectedRequest {
            name: self.hub.name.clone(),
            version: self.hub.version.clone(),
            platform: self.hub.platform.clone(),
            hub_id: self.hub.hub_id.clone(),
            token,
        };

        let (client, handshake) = WsClient::connect(&ws_url, &hub_req).await?;

        match handshake {
            HandshakeResult::Connected(status) => {
                self.setup_client_callbacks(&client, agent_id).await;

                let connected_agent = ConnectedAgent {
                    agent: agent.clone(),
                    status,
                };

                *self.ws_client.lock().await = Some(client);
                *self.connected.write().await =
                    Some((agent_id.to_string(), connected_agent.clone()));
                self.set_state(agent_id, ConnectionState::Connected).await;

                info!("connected to agent {}", agent_id);
                Ok(connected_agent)
            }
            HandshakeResult::NeedsPairing(pairing) => {
                // Store the client — it stays alive for confirm_pairing.
                *self.ws_client.lock().await = Some(client);
                *self.pairing_agent_id.lock().await = Some(agent_id.to_string());
                self.set_state(agent_id, ConnectionState::PairingRequired)
                    .await;

                let _ = self
                    .events_tx
                    .send(ConnectionEvent::PairingNeeded {
                        agent_id: agent_id.to_string(),
                        code: pairing.code.clone(),
                        expires_in: pairing.expires_in,
                    })
                    .await;

                Err(WsError::PairingFailed(format!(
                    "pairing required (code: {})",
                    pairing.code
                )))
            }
        }
    }

    /// Confirms a pairing code for an Agent that requires pairing.
    ///
    /// Must be called after receiving `ConnectionEvent::PairingNeeded`.
    /// Uses the existing WsClient that was kept alive from `connect_agent`.
    pub async fn confirm_pairing(
        &self,
        agent_id: &str,
        code: &str,
    ) -> Result<ConnectedAgent, WsError> {
        // Verify we have a pending pairing for this agent.
        let pairing_id = self.pairing_agent_id.lock().await.clone();
        if pairing_id.as_deref() != Some(agent_id) {
            return Err(WsError::PairingFailed("no pending pairing".into()));
        }

        // Use the existing client to confirm pairing.
        let client_guard = self.ws_client.lock().await;
        let client = client_guard.as_ref().ok_or(WsError::Closed)?;

        let success = client.confirm_pairing(code).await?;
        drop(client_guard);

        // Save token for future connections.
        if let Some(store) = &self.token_store {
            let _ = store.save_token(agent_id, &success.token);
        }

        // Clear pairing state.
        *self.pairing_agent_id.lock().await = None;

        // Close the pairing connection and reconnect with the token.
        if let Some(client) = self.ws_client.lock().await.take() {
            client.close().await;
        }

        // Reconnect — now the token will be used automatically.
        self.connect_agent(agent_id).await
    }

    /// Disconnects from the current Agent.
    pub async fn disconnect_agent(&self) {
        if let Some(client) = self.ws_client.lock().await.take() {
            client.close().await;
        }
        *self.pairing_agent_id.lock().await = None;
        if let Some((id, _)) = self.connected.write().await.take() {
            self.set_state(&id, ConnectionState::Disconnected).await;
            debug!("disconnected from agent {id}");
        }
    }

    /// Sends a request to the connected Agent.
    pub async fn send_request<T: serde::Serialize>(
        &self,
        msg_type: MessageType,
        payload: Option<&T>,
    ) -> Result<Message, WsError> {
        let client = self.ws_client.lock().await;
        let client = client.as_ref().ok_or(WsError::Closed)?;
        client.send_request(msg_type, payload).await
    }

    /// Sends binary data with a JSON header to the connected Agent.
    pub async fn send_binary(
        &self,
        header: &serde_json::Value,
        data: &[u8],
    ) -> Result<Message, WsError> {
        let client = self.ws_client.lock().await;
        let client = client.as_ref().ok_or(WsError::Closed)?;
        client.send_binary(header, data).await
    }

    /// Fetches full Agent info from the connected Agent.
    pub async fn get_info(&self) -> Result<InfoResponse, WsError> {
        let resp = self.send_request::<()>(MessageType::GetInfo, None).await?;
        let info: InfoResponse =
            resp.parse_payload::<InfoResponse>()?
                .ok_or_else(|| WsError::AgentError {
                    code: 500,
                    message: "empty info response".into(),
                })?;
        Ok(info)
    }

    /// Shuts down the connection manager.
    pub async fn shutdown(&self) {
        let _ = self.cancel_tx.send(true);
        self.disconnect_agent().await;
        info!("connection manager shut down");
    }

    /// Sets up event forwarding and disconnect callbacks on a WsClient.
    async fn setup_client_callbacks(&self, client: &WsClient, agent_id: &str) {
        // Event forwarding.
        let events_tx = self.events_tx.clone();
        let agent_id_str = agent_id.to_string();
        client
            .set_event_callback(Box::new(move |msg_type, message| {
                let _ = events_tx.try_send(ConnectionEvent::AgentEvent {
                    agent_id: agent_id_str.clone(),
                    msg_type,
                    message,
                });
            }))
            .await;

        // Disconnect handler.
        let events_tx = self.events_tx.clone();
        let state = self.state.clone();
        let connected = self.connected.clone();
        let agent_id_str = agent_id.to_string();
        client
            .set_disconnect_callback(Box::new(move || {
                let id = agent_id_str.clone();
                if let Ok(mut s) = state.try_write() {
                    s.insert(id.clone(), ConnectionState::Disconnected);
                }
                if let Ok(mut c) = connected.try_write() {
                    *c = None;
                }
                let _ = events_tx.try_send(ConnectionEvent::StateChanged {
                    agent_id: id,
                    state: ConnectionState::Disconnected,
                });
            }))
            .await;
    }

    /// Updates the connection state for an Agent and emits an event.
    async fn set_state(&self, agent_id: &str, new_state: ConnectionState) {
        self.state
            .write()
            .await
            .insert(agent_id.to_string(), new_state.clone());
        let _ = self
            .events_tx
            .send(ConnectionEvent::StateChanged {
                agent_id: agent_id.to_string(),
                state: new_state,
            })
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hub() -> HubIdentity {
        HubIdentity {
            name: "Test Hub".into(),
            version: "0.1.0".into(),
            platform: "linux".into(),
            hub_id: "hub-test-1".into(),
        }
    }

    #[tokio::test]
    async fn connection_manager_new() {
        let mgr = ConnectionManager::new(test_hub(), None);
        assert!(mgr.get_discovered().await.is_empty());
        assert!(mgr.get_connected().await.is_none());
    }

    #[tokio::test]
    async fn take_events_once() {
        let mgr = ConnectionManager::new(test_hub(), None);
        let rx1 = mgr.take_events().await;
        assert!(rx1.is_some());
        let rx2 = mgr.take_events().await;
        assert!(rx2.is_none());
    }

    #[tokio::test]
    async fn connect_unknown_agent_fails() {
        let mgr = ConnectionManager::new(test_hub(), None);
        let result = mgr.connect_agent("nonexistent-id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn disconnect_when_not_connected_is_noop() {
        let mgr = ConnectionManager::new(test_hub(), None);
        mgr.disconnect_agent().await;
    }

    #[tokio::test]
    async fn shutdown_is_clean() {
        let mgr = ConnectionManager::new(test_hub(), None);
        mgr.shutdown().await;
        mgr.shutdown().await;
    }

    #[tokio::test]
    async fn send_request_without_connection_fails() {
        let mgr = ConnectionManager::new(test_hub(), None);
        let result = mgr.send_request::<()>(MessageType::Ping, None).await;
        assert!(matches!(result, Err(WsError::Closed)));
    }

    #[tokio::test]
    async fn send_binary_without_connection_fails() {
        let mgr = ConnectionManager::new(test_hub(), None);
        let header = serde_json::json!({"type": "test"});
        let result = mgr.send_binary(&header, b"data").await;
        assert!(matches!(result, Err(WsError::Closed)));
    }

    #[tokio::test]
    async fn confirm_pairing_without_pending_fails() {
        let mgr = ConnectionManager::new(test_hub(), None);
        let result = mgr.confirm_pairing("agent-1", "123456").await;
        assert!(matches!(result, Err(WsError::PairingFailed(_))));
    }

    #[test]
    fn connection_state_equality() {
        assert_eq!(ConnectionState::Discovered, ConnectionState::Discovered);
        assert_ne!(ConnectionState::Connected, ConnectionState::Connecting);
    }

    #[test]
    fn hub_identity_clone() {
        let hub = test_hub();
        let hub2 = hub.clone();
        assert_eq!(hub.name, hub2.name);
        assert_eq!(hub.hub_id, hub2.hub_id);
    }
}
