//! WebSocket reconnection logic with exponential backoff.
//!
//! Contains the shared [`WsContext`], cancellation helpers, WebSocket callback
//! setup, and the reconnect loop.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::{Mutex, RwLock, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, trace, warn};

use capydeploy_discovery::client::Client as DiscoveryClient;
use capydeploy_discovery::types::DiscoveredAgent;
use capydeploy_protocol::messages::HubConnectedRequest;

use crate::pairing::TokenStore;
use crate::types::{
    ConnectedAgent, ConnectionEvent, ConnectionState, HubIdentity, MAX_NO_MDNS_ATTEMPTS,
    ReconnectConfig,
};
use crate::ws_client::{HandshakeResult, WsClient};

/// Shared state passed to free functions for WebSocket callback setup
/// and reconnection. Avoids threading 12 separate Arc parameters.
#[derive(Clone)]
pub(crate) struct WsContext {
    pub(crate) hub: HubIdentity,
    pub(crate) token_store: Option<Arc<TokenStore>>,
    pub(crate) discovered: Arc<RwLock<HashMap<String, DiscoveredAgent>>>,
    pub(crate) connected: Arc<RwLock<Option<(String, ConnectedAgent)>>>,
    pub(crate) ws_client: Arc<Mutex<Option<WsClient>>>,
    pub(crate) state: Arc<RwLock<HashMap<String, ConnectionState>>>,
    pub(crate) events_tx: mpsc::Sender<ConnectionEvent>,
    pub(crate) reconnect_cancel: Arc<std::sync::Mutex<Option<(String, CancellationToken)>>>,
    pub(crate) manual_disconnect: Arc<AtomicBool>,
    pub(crate) reconnect_config: ReconnectConfig,
    pub(crate) last_known_addr: Arc<Mutex<Option<(String, DiscoveredAgent)>>>,
}

/// Cancels any active reconnect loop regardless of agent ID.
pub(crate) fn cancel_any_reconnect(
    reconnect_cancel: &std::sync::Mutex<Option<(String, CancellationToken)>>,
) {
    if let Ok(mut guard) = reconnect_cancel.lock()
        && let Some((_, token)) = guard.take()
    {
        token.cancel();
    }
}

/// Cancels the reconnect loop only if it targets the given agent.
pub(crate) fn cancel_reconnect_for(
    reconnect_cancel: &std::sync::Mutex<Option<(String, CancellationToken)>>,
    agent_id: &str,
) {
    if let Ok(mut guard) = reconnect_cancel.lock() {
        let matches = guard.as_ref().is_some_and(|(id, _)| id == agent_id);
        if matches && let Some((_, token)) = guard.take() {
            token.cancel();
        }
    }
}

/// Sets up event forwarding and disconnect callbacks (with reconnect logic)
/// on a [`WsClient`].
pub(crate) async fn setup_ws_callbacks(client: &WsClient, agent_id: &str, ctx: WsContext) {
    // Event forwarding callback.
    let events_tx = ctx.events_tx.clone();
    let agent_id_ev = agent_id.to_string();
    client
        .set_event_callback(Box::new(move |msg_type, message| {
            trace!(msg_type = ?msg_type, agent = %agent_id_ev, "forwarding agent event to hub event loop");
            match events_tx.try_send(ConnectionEvent::AgentEvent {
                agent_id: agent_id_ev.clone(),
                msg_type,
                message,
            }) {
                Ok(()) => {}
                Err(e) => warn!("failed to forward agent event: {e}"),
            }
        }))
        .await;

    // Disconnect callback — handles manual, agent-revoked, and unexpected disconnects.
    let agent_id_dc = agent_id.to_string();
    let agent_closed = client.agent_closed();
    let ctx_dc = ctx;
    client
        .set_disconnect_callback(Box::new(move || {
            let id = agent_id_dc.clone();

            // Always clear connected state.
            if let Ok(mut c) = ctx_dc.connected.try_write() {
                *c = None;
            }

            let no_reconnect = ctx_dc.manual_disconnect.load(Ordering::Relaxed)
                || agent_closed.load(Ordering::Relaxed);

            if no_reconnect {
                // User-initiated or agent-revoked disconnect — no reconnect.
                if let Ok(mut s) = ctx_dc.state.try_write() {
                    s.insert(id.clone(), ConnectionState::Disconnected);
                }
                let _ = ctx_dc.events_tx.try_send(ConnectionEvent::StateChanged {
                    agent_id: id,
                    state: ConnectionState::Disconnected,
                });
            } else {
                // Unexpected disconnect — emit Disconnected then spawn reconnect.
                if let Ok(mut s) = ctx_dc.state.try_write() {
                    s.insert(id.clone(), ConnectionState::Disconnected);
                }
                let _ = ctx_dc.events_tx.try_send(ConnectionEvent::StateChanged {
                    agent_id: id.clone(),
                    state: ConnectionState::Disconnected,
                });

                // Create a cancellation token and store it.
                let cancel = CancellationToken::new();
                cancel_any_reconnect(&ctx_dc.reconnect_cancel);
                if let Ok(mut guard) = ctx_dc.reconnect_cancel.lock() {
                    *guard = Some((id.clone(), cancel.clone()));
                }

                tokio::spawn(reconnect_loop(id, ctx_dc.clone(), cancel));
            }
        }))
        .await;
}

/// Reconnection loop with exponential backoff.
///
/// Returns a boxed future to break the recursive type cycle with
/// `setup_ws_callbacks` (which spawns this function from its disconnect
/// callback).
pub(crate) fn reconnect_loop(
    agent_id: String,
    ctx: WsContext,
    cancel: CancellationToken,
) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(async move {
        // Inline mDNS refresh — catch quick agent restarts before the continuous
        // discovery loop notices them. Only updates the cache, no UI events.
        let refresh_client = DiscoveryClient::new();
        if let Ok(agents) = refresh_client.discover(Duration::from_secs(1)).await {
            let mut disc = ctx.discovered.write().await;
            for agent in agents {
                disc.insert(agent.info.id.clone(), agent);
            }
        }

        let mut attempt: u32 = 0;
        let mut no_mdns_count: u32 = 0;

        loop {
            attempt = attempt.saturating_add(1);
            let delay = ctx.reconnect_config.delay_for_attempt(attempt);
            let delay_secs = delay.as_secs_f64();

            // Update state and emit Reconnecting event.
            ctx.state
                .write()
                .await
                .insert(agent_id.clone(), ConnectionState::Reconnecting { attempt });
            let _ = ctx
                .events_tx
                .send(ConnectionEvent::Reconnecting {
                    agent_id: agent_id.clone(),
                    attempt,
                    next_retry_secs: delay_secs,
                })
                .await;

            info!(
                agent = %agent_id,
                attempt,
                delay_secs = format_args!("{delay_secs:.1}"),
                "reconnecting"
            );

            // Wait for the backoff delay (or cancellation).
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!(agent = %agent_id, "reconnect cancelled");
                    return;
                }
                _ = tokio::time::sleep(delay) => {}
            }

            // Check cancellation after sleep.
            if cancel.is_cancelled() {
                return;
            }

            // Resolve the WebSocket URL: prefer mDNS, fall back to last known address.
            let discovered_agent = ctx.discovered.read().await.get(&agent_id).cloned();
            let (ws_url, fallback_agent) = if let Some(agent) = discovered_agent {
                no_mdns_count = 0;
                (agent.websocket_address(), agent)
            } else {
                no_mdns_count += 1;
                if no_mdns_count > MAX_NO_MDNS_ATTEMPTS {
                    info!(agent = %agent_id, "too many attempts without mDNS, stopping reconnect");
                    ctx.state
                        .write()
                        .await
                        .insert(agent_id.clone(), ConnectionState::Disconnected);
                    let _ = ctx
                        .events_tx
                        .send(ConnectionEvent::StateChanged {
                            agent_id: agent_id.clone(),
                            state: ConnectionState::Disconnected,
                        })
                        .await;
                    break;
                }
                match ctx.last_known_addr.lock().await.clone() {
                    Some((addr, agent)) => {
                        debug!(agent = %agent_id, no_mdns_count, "using last known address");
                        (addr, agent)
                    }
                    None => {
                        info!(agent = %agent_id, "no mDNS and no last known address, stopping reconnect");
                        ctx.state
                            .write()
                            .await
                            .insert(agent_id.clone(), ConnectionState::Disconnected);
                        let _ = ctx
                            .events_tx
                            .send(ConnectionEvent::StateChanged {
                                agent_id: agent_id.clone(),
                                state: ConnectionState::Disconnected,
                            })
                            .await;
                        break;
                    }
                }
            };
            let token = ctx
                .token_store
                .as_ref()
                .and_then(|s| s.get_token(&agent_id))
                .unwrap_or_default();

            let hub_req = HubConnectedRequest {
                name: ctx.hub.name.clone(),
                version: ctx.hub.version.clone(),
                platform: ctx.hub.platform.clone(),
                hub_id: ctx.hub.hub_id.clone(),
                token,
                protocol_version: capydeploy_protocol::constants::PROTOCOL_VERSION,
            };

            match WsClient::connect(&ws_url, &hub_req).await {
                Ok((client, HandshakeResult::Connected(status))) => {
                    // Set up callbacks on the new client (including reconnect on future disconnect).
                    setup_ws_callbacks(&client, &agent_id, ctx.clone()).await;

                    let connected_agent = ConnectedAgent {
                        agent: fallback_agent.clone(),
                        status,
                    };

                    *ctx.ws_client.lock().await = Some(client);
                    *ctx.connected.write().await = Some((agent_id.clone(), connected_agent));
                    *ctx.last_known_addr.lock().await = Some((ws_url, fallback_agent));
                    ctx.state
                        .write()
                        .await
                        .insert(agent_id.clone(), ConnectionState::Connected);

                    let _ = ctx
                        .events_tx
                        .send(ConnectionEvent::StateChanged {
                            agent_id: agent_id.clone(),
                            state: ConnectionState::Connected,
                        })
                        .await;

                    info!(agent = %agent_id, "reconnected successfully");
                    break;
                }
                Ok((client, HandshakeResult::NeedsPairing(_))) => {
                    // Token invalid — user must re-pair manually.
                    client.close().await;
                    warn!(agent = %agent_id, "agent requires re-pairing, stopping reconnect");
                    ctx.state
                        .write()
                        .await
                        .insert(agent_id.clone(), ConnectionState::Disconnected);
                    let _ = ctx
                        .events_tx
                        .send(ConnectionEvent::StateChanged {
                            agent_id: agent_id.clone(),
                            state: ConnectionState::Disconnected,
                        })
                        .await;
                    break;
                }
                Err(e) => {
                    warn!(
                        agent = %agent_id,
                        attempt,
                        error = %e,
                        "reconnect attempt failed"
                    );
                    // Continue loop — next attempt with increased backoff.
                }
            }

            // Check cancellation after connect attempt.
            if cancel.is_cancelled() {
                return;
            }
        }

        // Clean up the cancel token if it's still ours.
        if let Ok(mut guard) = ctx.reconnect_cancel.lock() {
            let ours = guard.as_ref().is_some_and(|(id, _)| id == &agent_id);
            if ours {
                *guard = None;
            }
        }
    }) // Box::pin
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_any_reconnect_clears_token() {
        let cancel = Arc::new(std::sync::Mutex::new(None));
        let token = CancellationToken::new();
        *cancel.lock().unwrap() = Some(("agent-1".into(), token.clone()));

        cancel_any_reconnect(&cancel);

        assert!(cancel.lock().unwrap().is_none());
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancel_reconnect_for_only_targets_matching_agent() {
        let cancel = Arc::new(std::sync::Mutex::new(None));
        let token = CancellationToken::new();
        *cancel.lock().unwrap() = Some(("agent-1".into(), token.clone()));

        // Wrong agent — should not cancel.
        cancel_reconnect_for(&cancel, "agent-2");
        assert!(cancel.lock().unwrap().is_some());
        assert!(!token.is_cancelled());

        // Right agent — should cancel.
        cancel_reconnect_for(&cancel, "agent-1");
        assert!(cancel.lock().unwrap().is_none());
        assert!(token.is_cancelled());
    }
}
