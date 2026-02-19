//! Connection manager orchestrating discovery and WebSocket connections.
//!
//! Auto-discovers Agents via mDNS, manages WebSocket client lifecycles,
//! tracks connection state, and reconnects automatically with exponential
//! backoff on unexpected disconnects.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{Mutex, RwLock, mpsc, watch};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use capydeploy_discovery::client::Client as DiscoveryClient;
use capydeploy_discovery::types::DiscoveredAgent;
use capydeploy_protocol::constants::{
    self, MessageType, PROTOCOL_VERSION, check_protocol_compatibility,
};
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages::{HubConnectedRequest, InfoResponse};

use crate::pairing::TokenStore;
use crate::reconnection::{WsContext, cancel_any_reconnect, setup_ws_callbacks};
use crate::types::{
    ConnectedAgent, ConnectionEvent, ConnectionState, HubIdentity, ReconnectConfig,
};
use crate::ws_client::{HandshakeResult, WsClient, WsError};

/// Connection manager for discovering and connecting to Agents.
pub struct ConnectionManager {
    pub(crate) hub: HubIdentity,
    pub(crate) discovery: Arc<Mutex<DiscoveryClient>>,
    pub(crate) token_store: Option<Arc<TokenStore>>,
    pub(crate) discovered: Arc<RwLock<HashMap<String, DiscoveredAgent>>>,
    pub(crate) connected: Arc<RwLock<Option<(String, ConnectedAgent)>>>,
    pub(crate) ws_client: Arc<Mutex<Option<WsClient>>>,
    /// Agent ID for a connection in pairing state.
    pub(crate) pairing_agent_id: Arc<Mutex<Option<String>>>,
    pub(crate) events_tx: mpsc::Sender<ConnectionEvent>,
    pub(crate) events_rx: Mutex<Option<mpsc::Receiver<ConnectionEvent>>>,
    pub(crate) cancel_tx: watch::Sender<bool>,
    pub(crate) cancel_rx: watch::Receiver<bool>,
    pub(crate) state: Arc<RwLock<HashMap<String, ConnectionState>>>,
    /// Cancel token for the active reconnect loop, keyed by agent ID.
    pub(crate) reconnect_cancel: Arc<std::sync::Mutex<Option<(String, CancellationToken)>>>,
    /// Set to true when the user explicitly disconnects.
    pub(crate) manual_disconnect: Arc<AtomicBool>,
    /// Reconnection backoff configuration.
    pub(crate) reconnect_config: ReconnectConfig,
    /// Last successfully connected WebSocket URL for reconnect fallback.
    pub(crate) last_known_addr: Arc<Mutex<Option<(String, DiscoveredAgent)>>>,
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
            reconnect_cancel: Arc::new(std::sync::Mutex::new(None)),
            manual_disconnect: Arc::new(AtomicBool::new(false)),
            reconnect_config: ReconnectConfig::default(),
            last_known_addr: Arc::new(Mutex::new(None)),
        }
    }

    /// Takes the event receiver. Can only be called once.
    pub async fn take_events(&self) -> Option<mpsc::Receiver<ConnectionEvent>> {
        self.events_rx.lock().await.take()
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
        // Cancel any active reconnect and reset manual disconnect flag.
        cancel_any_reconnect(&self.reconnect_cancel);
        self.manual_disconnect.store(false, Ordering::Relaxed);

        // Find the discovered agent.
        let agent = self
            .discovered
            .read()
            .await
            .get(agent_id)
            .cloned()
            .ok_or(WsError::Closed)?;

        // Disconnect existing connection (without setting manual flag).
        self.disconnect_agent_inner(false).await;

        self.set_state(agent_id, ConnectionState::Connecting).await;

        let ws_url = agent.websocket_address();
        info!(
            agent = %agent_id,
            url = %ws_url,
            ips = ?agent.ips,
            host = %agent.host,
            "connecting to agent"
        );
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
            protocol_version: PROTOCOL_VERSION,
        };

        let (client, handshake) = match WsClient::connect(&ws_url, &hub_req).await {
            Ok(r) => r,
            Err(e) => {
                warn!(agent = %agent_id, error = %e, "connection failed");
                self.set_state(agent_id, ConnectionState::Disconnected)
                    .await;
                return Err(e);
            }
        };

        match handshake {
            HandshakeResult::Connected(status) => {
                // Check protocol compatibility before accepting.
                match check_protocol_compatibility(status.protocol_version) {
                    constants::ProtocolCompatibility::Incompatible {
                        peer_version,
                        reason,
                    } => {
                        warn!(
                            agent = %agent_id,
                            peer_version,
                            "protocol incompatible: {reason}"
                        );
                        client.close().await;
                        self.set_state(agent_id, ConnectionState::Disconnected)
                            .await;
                        return Err(WsError::AgentError {
                            code: 406,
                            message: reason,
                        });
                    }
                    constants::ProtocolCompatibility::Deprecated { peer_version } => {
                        let msg = format!(
                            "agent protocol v{peer_version} is deprecated \
                             (current: v{PROTOCOL_VERSION})"
                        );
                        warn!(agent = %agent_id, "{msg}");
                        let _ = self
                            .events_tx
                            .send(ConnectionEvent::ProtocolWarning {
                                agent_id: agent_id.to_string(),
                                message: msg,
                            })
                            .await;
                    }
                    constants::ProtocolCompatibility::Compatible => {}
                }

                self.setup_client_callbacks(&client, agent_id).await;

                let connected_agent = ConnectedAgent {
                    agent: agent.clone(),
                    status,
                };

                *self.ws_client.lock().await = Some(client);
                *self.connected.write().await =
                    Some((agent_id.to_string(), connected_agent.clone()));
                *self.last_known_addr.lock().await = Some((ws_url, agent.clone()));
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

    /// Disconnects from the current Agent (user-initiated).
    pub async fn disconnect_agent(&self) {
        self.disconnect_agent_inner(true).await;
    }

    /// Cancels any active reconnect loop and sets the agent to Disconnected.
    pub async fn cancel_all_reconnects(&self) {
        cancel_any_reconnect(&self.reconnect_cancel);
        // Set any Reconnecting agents to Disconnected.
        let mut states = self.state.write().await;
        for (_, state) in states.iter_mut() {
            if matches!(state, ConnectionState::Reconnecting { .. }) {
                *state = ConnectionState::Disconnected;
            }
        }
    }

    /// Internal disconnect. When `set_manual` is true, sets the manual
    /// disconnect flag so the disconnect callback won't trigger reconnect.
    async fn disconnect_agent_inner(&self, set_manual: bool) {
        if set_manual {
            self.manual_disconnect.store(true, Ordering::Relaxed);
            cancel_any_reconnect(&self.reconnect_cancel);
        }
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
        cancel_any_reconnect(&self.reconnect_cancel);
        self.disconnect_agent_inner(true).await;
        info!("connection manager shut down");
    }

    /// Builds a [`WsContext`] from the current manager state.
    pub(crate) fn ws_context(&self) -> WsContext {
        WsContext {
            hub: self.hub.clone(),
            token_store: self.token_store.clone(),
            discovered: self.discovered.clone(),
            connected: self.connected.clone(),
            ws_client: self.ws_client.clone(),
            state: self.state.clone(),
            events_tx: self.events_tx.clone(),
            reconnect_cancel: self.reconnect_cancel.clone(),
            manual_disconnect: self.manual_disconnect.clone(),
            reconnect_config: self.reconnect_config.clone(),
            last_known_addr: self.last_known_addr.clone(),
        }
    }

    /// Sets up event forwarding and disconnect callbacks on a WsClient.
    async fn setup_client_callbacks(&self, client: &WsClient, agent_id: &str) {
        setup_ws_callbacks(client, agent_id, self.ws_context()).await;
    }

    /// Updates the connection state for an Agent and emits an event.
    pub(crate) async fn set_state(&self, agent_id: &str, new_state: ConnectionState) {
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
}
