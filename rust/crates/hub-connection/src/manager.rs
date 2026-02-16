//! Connection manager orchestrating discovery and WebSocket connections.
//!
//! Auto-discovers Agents via mDNS, manages WebSocket client lifecycles,
//! tracks connection state, and reconnects automatically with exponential
//! backoff on unexpected disconnects.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, RwLock, mpsc, watch};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

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
    /// Connection lost, attempting to reconnect.
    Reconnecting { attempt: u32 },
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
    /// Reconnection is in progress for an Agent.
    Reconnecting {
        agent_id: String,
        attempt: u32,
        next_retry_secs: f64,
    },
}

/// Configuration for automatic reconnection with exponential backoff.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Initial delay before the first reconnection attempt.
    pub initial_delay: Duration,
    /// Maximum delay between attempts (backoff cap).
    pub max_delay: Duration,
    /// Multiplier for each subsequent attempt.
    pub backoff_factor: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(15),
            backoff_factor: 2.0,
        }
    }
}

impl ReconnectConfig {
    /// Calculates the delay for a given attempt number (1-based),
    /// with ±25% jitter to avoid thundering herd.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp = attempt.saturating_sub(1).min(63) as i32;
        let secs = self.initial_delay.as_secs_f64() * self.backoff_factor.powi(exp);
        let capped = secs.min(self.max_delay.as_secs_f64());
        // Add ±25% jitter.
        let jitter = capped * 0.25;
        let offset = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as f64
            / u32::MAX as f64)
            * 2.0
            - 1.0; // [-1.0, 1.0)
        let with_jitter = (capped + jitter * offset).max(0.05);
        Duration::from_secs_f64(with_jitter)
    }
}

/// Hub identity used in connection handshakes.
#[derive(Debug, Clone)]
pub struct HubIdentity {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub hub_id: String,
}

/// Maximum reconnect attempts without mDNS visibility before giving up.
const MAX_NO_MDNS_ATTEMPTS: u32 = 30;

/// Shared state passed to free functions for WebSocket callback setup
/// and reconnection. Avoids threading 12 separate Arc parameters.
#[derive(Clone)]
struct WsContext {
    hub: HubIdentity,
    token_store: Option<Arc<TokenStore>>,
    discovered: Arc<RwLock<HashMap<String, DiscoveredAgent>>>,
    connected: Arc<RwLock<Option<(String, ConnectedAgent)>>>,
    ws_client: Arc<Mutex<Option<WsClient>>>,
    state: Arc<RwLock<HashMap<String, ConnectionState>>>,
    events_tx: mpsc::Sender<ConnectionEvent>,
    reconnect_cancel: Arc<std::sync::Mutex<Option<(String, CancellationToken)>>>,
    manual_disconnect: Arc<AtomicBool>,
    reconnect_config: ReconnectConfig,
    last_known_addr: Arc<Mutex<Option<(String, DiscoveredAgent)>>>,
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
    /// Cancel token for the active reconnect loop, keyed by agent ID.
    reconnect_cancel: Arc<std::sync::Mutex<Option<(String, CancellationToken)>>>,
    /// Set to true when the user explicitly disconnects.
    manual_disconnect: Arc<AtomicBool>,
    /// Reconnection backoff configuration.
    reconnect_config: ReconnectConfig,
    /// Last successfully connected WebSocket URL for reconnect fallback.
    last_known_addr: Arc<Mutex<Option<(String, DiscoveredAgent)>>>,
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
        };

        let (client, handshake) = match WsClient::connect(&ws_url, &hub_req).await {
            Ok(r) => r,
            Err(e) => {
                warn!(agent = %agent_id, error = %e, "connection failed");
                self.set_state(agent_id, ConnectionState::Disconnected).await;
                return Err(e);
            }
        };

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
    fn ws_context(&self) -> WsContext {
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

// ---------------------------------------------------------------------------
// Free functions for reconnection
// ---------------------------------------------------------------------------

/// Cancels any active reconnect loop regardless of agent ID.
fn cancel_any_reconnect(
    reconnect_cancel: &std::sync::Mutex<Option<(String, CancellationToken)>>,
) {
    if let Ok(mut guard) = reconnect_cancel.lock()
        && let Some((_, token)) = guard.take()
    {
        token.cancel();
    }
}

/// Cancels the reconnect loop only if it targets the given agent.
fn cancel_reconnect_for(
    reconnect_cancel: &std::sync::Mutex<Option<(String, CancellationToken)>>,
    agent_id: &str,
) {
    if let Ok(mut guard) = reconnect_cancel.lock() {
        let matches = guard.as_ref().is_some_and(|(id, _)| id == agent_id);
        if matches
            && let Some((_, token)) = guard.take()
        {
            token.cancel();
        }
    }
}

/// Sets up event forwarding and disconnect callbacks (with reconnect logic)
/// on a [`WsClient`].
async fn setup_ws_callbacks(client: &WsClient, agent_id: &str, ctx: WsContext) {
    // Event forwarding callback.
    let events_tx = ctx.events_tx.clone();
    let agent_id_ev = agent_id.to_string();
    client
        .set_event_callback(Box::new(move |msg_type, message| {
            let _ = events_tx.try_send(ConnectionEvent::AgentEvent {
                agent_id: agent_id_ev.clone(),
                msg_type,
                message,
            });
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
fn reconnect_loop(
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
                *ctx.connected.write().await =
                    Some((agent_id.clone(), connected_agent));
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
        assert_eq!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 1 },
        );
        assert_ne!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 2 },
        );
    }

    #[test]
    fn hub_identity_clone() {
        let hub = test_hub();
        let hub2 = hub.clone();
        assert_eq!(hub.name, hub2.name);
        assert_eq!(hub.hub_id, hub2.hub_id);
    }

    #[test]
    fn reconnect_config_defaults() {
        let config = ReconnectConfig::default();
        assert_eq!(config.initial_delay, Duration::from_millis(250));
        assert_eq!(config.max_delay, Duration::from_secs(15));
        assert!((config.backoff_factor - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reconnect_config_delay_backoff() {
        let config = ReconnectConfig::default();
        // Base delays: 250ms, 500ms, 1s, 2s, 4s, 8s, 15s (capped), 15s...
        // With ±25% jitter, check that values are within expected range.
        let expected_base = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 15.0, 15.0];
        for (i, &base) in expected_base.iter().enumerate() {
            let delay = config.delay_for_attempt((i + 1) as u32);
            let secs = delay.as_secs_f64();
            let lo = base * 0.74; // -26% to allow for jitter rounding
            let hi = base * 1.26; // +26%
            assert!(
                secs >= lo && secs <= hi,
                "attempt {}: {secs:.3}s not in [{lo:.3}, {hi:.3}]",
                i + 1
            );
        }
    }

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
