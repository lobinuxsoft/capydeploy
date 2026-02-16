//! WebSocket client for Hub-to-Agent communication.
//!
//! Implements request-response pattern with UUID correlation,
//! ping/pong keepalive, and push event dispatching.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::tungstenite;
use tracing::{debug, error, trace, warn};

use capydeploy_protocol::constants::{
    MessageType, WS_BINARY_REQUEST_TIMEOUT, WS_MAX_MESSAGE_SIZE, WS_PING_PERIOD, WS_PONG_WAIT,
    WS_REQUEST_TIMEOUT,
};
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages::{
    AgentStatusResponse, HubConnectedRequest, PairConfirmRequest, PairFailedResponse,
    PairSuccessResponse, PairingRequiredResponse,
};

/// Errors from the WebSocket client.
#[derive(Debug, thiserror::Error)]
pub enum WsError {
    #[error("WebSocket error: {0}")]
    Ws(#[from] tungstenite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("request timed out")]
    Timeout,

    #[error("connection closed")]
    Closed,

    #[error("pairing failed: {0}")]
    PairingFailed(String),

    #[error("agent error {code}: {message}")]
    AgentError { code: i32, message: String },
}

/// Result of the initial handshake with an Agent.
pub enum HandshakeResult {
    /// Connected and authenticated successfully.
    Connected(AgentStatusResponse),
    /// Agent requires pairing before allowing access.
    NeedsPairing(PairingRequiredResponse),
}

/// Callback type for push events from the Agent.
pub type EventCallback = Box<dyn Fn(MessageType, Message) + Send + Sync>;

/// Callback type for disconnect notification.
type DisconnectCallback = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;

/// WebSocket client connected to a single Agent.
///
/// The client is returned from [`WsClient::connect`] regardless of whether
/// the handshake succeeds or pairing is required. Check [`HandshakeResult`]
/// to determine the next step.
pub struct WsClient {
    write_tx: mpsc::Sender<tungstenite::Message>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Message>>>>,
    on_event: Arc<Mutex<Option<EventCallback>>>,
    on_disconnect: DisconnectCallback,
    /// Set to `true` by the read pump when the Agent sends a close frame
    /// with [`WS_CLOSE_TOKEN_REVOKED`]. The disconnect callback checks
    /// this to suppress automatic reconnection.
    agent_closed: Arc<AtomicBool>,
    _read_handle: tokio::task::JoinHandle<()>,
    _write_handle: tokio::task::JoinHandle<()>,
    _ping_handle: tokio::task::JoinHandle<()>,
    cancel: tokio_util::sync::CancellationToken,
}

impl WsClient {
    /// Connects to an Agent WebSocket and performs the initial handshake.
    ///
    /// The client is **always** returned. Check [`HandshakeResult`] to see
    /// if the connection is ready or if pairing is needed first.
    pub async fn connect(
        url: &str,
        hub_request: &HubConnectedRequest,
    ) -> Result<(Self, HandshakeResult), WsError> {
        let mut ws_config =
            tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
        ws_config.max_message_size = Some(WS_MAX_MESSAGE_SIZE);
        ws_config.max_frame_size = Some(WS_MAX_MESSAGE_SIZE);
        let (ws_stream, _) =
            tokio_tungstenite::connect_async_with_config(url, Some(ws_config), false).await?;
        let (write, read) = ws_stream.split();

        let (write_tx, write_rx) = mpsc::channel::<tungstenite::Message>(256);
        let pending: Arc<Mutex<HashMap<String, oneshot::Sender<Message>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let on_event: Arc<Mutex<Option<EventCallback>>> = Arc::new(Mutex::new(None));
        let on_disconnect: DisconnectCallback = Arc::new(Mutex::new(None));
        let agent_closed = Arc::new(AtomicBool::new(false));
        let cancel = tokio_util::sync::CancellationToken::new();

        let write_handle = {
            let cancel = cancel.clone();
            tokio::spawn(write_pump(write, write_rx, cancel))
        };

        let read_handle = {
            let pending = pending.clone();
            let on_event = on_event.clone();
            let on_disconnect = on_disconnect.clone();
            let agent_closed = agent_closed.clone();
            let cancel = cancel.clone();
            let write_tx = write_tx.clone();
            tokio::spawn(read_pump(
                read, pending, on_event, on_disconnect, agent_closed, write_tx, cancel,
            ))
        };

        let ping_handle = {
            let write_tx = write_tx.clone();
            let cancel = cancel.clone();
            tokio::spawn(ping_pump(write_tx, cancel))
        };

        let client = Self {
            write_tx,
            pending,
            on_event,
            on_disconnect,
            agent_closed,
            _read_handle: read_handle,
            _write_handle: write_handle,
            _ping_handle: ping_handle,
            cancel,
        };

        // Perform handshake.
        let resp = client
            .send_request(MessageType::HubConnected, Some(hub_request))
            .await?;

        // Check if pairing is required — return client alive.
        if resp.msg_type == MessageType::PairingRequired
            && let Ok(Some(pairing)) = resp.parse_payload::<PairingRequiredResponse>()
        {
            return Ok((client, HandshakeResult::NeedsPairing(pairing)));
        }

        // Check for error response.
        if let Some(err) = &resp.error {
            return Err(WsError::AgentError {
                code: err.code,
                message: err.message.clone(),
            });
        }

        // Parse agent status.
        let status: AgentStatusResponse =
            resp.parse_payload::<AgentStatusResponse>()?
                .ok_or_else(|| WsError::AgentError {
                    code: 500,
                    message: "empty agent status".into(),
                })?;

        Ok((client, HandshakeResult::Connected(status)))
    }

    /// Sends a request and waits for the response.
    pub async fn send_request<T: serde::Serialize>(
        &self,
        msg_type: MessageType,
        payload: Option<&T>,
    ) -> Result<Message, WsError> {
        let id = uuid::Uuid::new_v4().to_string();
        let msg = Message::new(&id, msg_type, payload)?;
        let json = serde_json::to_string(&msg)?;

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);

        self.write_tx
            .send(tungstenite::Message::Text(json.into()))
            .await
            .map_err(|_| WsError::Closed)?;

        let result = tokio::time::timeout(WS_REQUEST_TIMEOUT, rx).await;

        // Clean up pending entry on any exit path.
        self.pending.lock().await.remove(&id);

        match result {
            Ok(Ok(resp)) => {
                if let Some(err) = &resp.error {
                    return Err(WsError::AgentError {
                        code: err.code,
                        message: err.message.clone(),
                    });
                }
                Ok(resp)
            }
            Ok(Err(_)) => Err(WsError::Closed),
            Err(_) => Err(WsError::Timeout),
        }
    }

    /// Confirms a pairing code with the Agent.
    ///
    /// Call this after receiving [`HandshakeResult::NeedsPairing`].
    /// The connection must still be alive (same client instance).
    pub async fn confirm_pairing(&self, code: &str) -> Result<PairSuccessResponse, WsError> {
        let req = PairConfirmRequest {
            code: code.to_string(),
        };
        let resp = self
            .send_request(MessageType::PairConfirm, Some(&req))
            .await?;

        match resp.msg_type {
            MessageType::PairSuccess => {
                let success: PairSuccessResponse = resp
                    .parse_payload::<PairSuccessResponse>()?
                    .ok_or_else(|| WsError::PairingFailed("empty response".into()))?;
                Ok(success)
            }
            MessageType::PairFailed => {
                let failed = resp.parse_payload::<PairFailedResponse>()?;
                let reason = failed.map(|f| f.reason).unwrap_or_default();
                Err(WsError::PairingFailed(reason))
            }
            _ => Err(WsError::PairingFailed("unexpected response".into())),
        }
    }

    /// Sets the callback for push events from the Agent.
    pub async fn set_event_callback(&self, cb: EventCallback) {
        *self.on_event.lock().await = Some(cb);
    }

    /// Sets the callback for disconnection.
    pub async fn set_disconnect_callback(&self, cb: Box<dyn Fn() + Send + Sync>) {
        *self.on_disconnect.lock().await = Some(cb);
    }

    /// Sends binary data with a JSON header and waits for the text response.
    ///
    /// Wire format (matching the Go Hub):
    /// `[4 bytes big-endian header length][JSON header bytes][binary data]`
    ///
    /// A UUID is injected into the header for request-response correlation.
    pub async fn send_binary(
        &self,
        header: &serde_json::Value,
        data: &[u8],
    ) -> Result<Message, WsError> {
        let id = uuid::Uuid::new_v4().to_string();

        // Inject the request ID into the header.
        let mut header = header.clone();
        if let Some(obj) = header.as_object_mut() {
            obj.insert("id".into(), serde_json::Value::String(id.clone()));
        }

        let header_bytes = serde_json::to_vec(&header)?;
        let header_len = header_bytes.len();

        // Build wire frame: [4 BE bytes][header JSON][data].
        let mut frame = Vec::with_capacity(4 + header_len + data.len());
        frame.push((header_len >> 24) as u8);
        frame.push((header_len >> 16) as u8);
        frame.push((header_len >> 8) as u8);
        frame.push(header_len as u8);
        frame.extend_from_slice(&header_bytes);
        frame.extend_from_slice(data);

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);

        self.write_tx
            .send(tungstenite::Message::Binary(frame.into()))
            .await
            .map_err(|_| WsError::Closed)?;

        // Binary transfers use a longer timeout to handle slow disk I/O
        // and network conditions during large chunk uploads.
        let result = tokio::time::timeout(WS_BINARY_REQUEST_TIMEOUT, rx).await;
        self.pending.lock().await.remove(&id);

        match result {
            Ok(Ok(resp)) => {
                if let Some(err) = &resp.error {
                    return Err(WsError::AgentError {
                        code: err.code,
                        message: err.message.clone(),
                    });
                }
                Ok(resp)
            }
            Ok(Err(_)) => Err(WsError::Closed),
            Err(_) => Err(WsError::Timeout),
        }
    }

    /// Returns `true` if the Agent sent a close frame with
    /// [`WS_CLOSE_TOKEN_REVOKED`], meaning reconnection should be suppressed.
    pub fn agent_closed(&self) -> Arc<AtomicBool> {
        self.agent_closed.clone()
    }

    /// Gracefully closes the connection.
    pub async fn close(&self) {
        self.cancel.cancel();
        let _ = self.write_tx.send(tungstenite::Message::Close(None)).await;
    }
}

impl Drop for WsClient {
    fn drop(&mut self) {
        self.cancel.cancel();
        self._read_handle.abort();
        self._write_handle.abort();
        self._ping_handle.abort();
    }
}

// ---------------------------------------------------------------------------
// Internal async pumps
// ---------------------------------------------------------------------------

/// Reads messages from the WebSocket and dispatches them.
///
/// Uses a pong deadline to detect dead connections: if no pong arrives
/// within [`WS_PONG_WAIT`] after a ping was sent, the connection is
/// considered dead and the loop exits (triggering reconnect).
async fn read_pump<S>(
    mut read: S,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Message>>>>,
    on_event: Arc<Mutex<Option<EventCallback>>>,
    on_disconnect: DisconnectCallback,
    agent_closed: Arc<AtomicBool>,
    write_tx: mpsc::Sender<tungstenite::Message>,
    cancel: tokio_util::sync::CancellationToken,
) where
    S: StreamExt<Item = Result<tungstenite::Message, tungstenite::Error>> + Unpin,
{
    // Pong deadline: any incoming message (not just Pong) resets the timer,
    // matching Go's SetReadDeadline behavior. If nothing arrives within
    // WS_PONG_WAIT the connection is considered dead.
    let pong_deadline = tokio::time::sleep(WS_PONG_WAIT);
    tokio::pin!(pong_deadline);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,

            () = &mut pong_deadline => {
                warn!("pong timeout — connection dead, closing");
                break;
            }

            msg = read.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        // ANY incoming message resets the deadline (matches Go behavior).
                        pong_deadline.as_mut().reset(tokio::time::Instant::now() + WS_PONG_WAIT);

                        match msg {
                            tungstenite::Message::Text(text) => {
                                handle_text_message(&text, &pending, &on_event).await;
                            }
                            tungstenite::Message::Ping(data) => {
                                trace!("received ping, sending pong");
                                let _ = write_tx.send(tungstenite::Message::Pong(data)).await;
                            }
                            tungstenite::Message::Pong(_) => {
                                trace!("received pong");
                            }
                            tungstenite::Message::Close(frame) => {
                                if let Some(ref f) = frame {
                                    use capydeploy_protocol::constants::WS_CLOSE_TOKEN_REVOKED;
                                    if u16::from(f.code) == WS_CLOSE_TOKEN_REVOKED {
                                        debug!("agent revoked token (close code {WS_CLOSE_TOKEN_REVOKED})");
                                        agent_closed.store(true, Ordering::Relaxed);
                                    }
                                }
                                debug!("received close frame");
                                break;
                            }
                            _ => {} // Binary — ignore
                        }
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket read error: {e}");
                        break;
                    }
                    None => {
                        debug!("WebSocket stream ended");
                        break;
                    }
                }
            }
        }
    }

    if let Some(cb) = on_disconnect.lock().await.as_ref() {
        cb();
    }
}

/// Handles a text message from the WebSocket.
async fn handle_text_message(
    text: &str,
    pending: &Arc<Mutex<HashMap<String, oneshot::Sender<Message>>>>,
    on_event: &Arc<Mutex<Option<EventCallback>>>,
) {
    if text.len() > WS_MAX_MESSAGE_SIZE {
        warn!("message too large ({} bytes), dropping", text.len());
        return;
    }

    let msg: Message = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            warn!("failed to parse message: {e}");
            return;
        }
    };

    trace!(msg_type = ?msg.msg_type, id = %msg.id, "received message");

    // Route response to pending request.
    let mut map = pending.lock().await;
    if let Some(tx) = map.remove(&msg.id) {
        let _ = tx.send(msg);
        return;
    }
    drop(map);

    // Push event to callback.
    let guard = on_event.lock().await;
    if let Some(cb) = guard.as_ref() {
        let msg_type = msg.msg_type.clone();
        cb(msg_type, msg);
    } else {
        warn!(msg_type = ?msg.msg_type, id = %msg.id, "no event callback set — dropping push event");
    }
}

/// Writes messages to the WebSocket.
async fn write_pump<S>(
    mut write: S,
    mut write_rx: mpsc::Receiver<tungstenite::Message>,
    cancel: tokio_util::sync::CancellationToken,
) where
    S: SinkExt<tungstenite::Message, Error = tungstenite::Error> + Unpin,
{
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            msg = write_rx.recv() => {
                match msg {
                    Some(m) => {
                        if let Err(e) = write.send(m).await {
                            error!("WebSocket write error: {e}");
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    let _ = write.send(tungstenite::Message::Close(None)).await;
}

/// Sends periodic pings to keep the connection alive.
async fn ping_pump(
    write_tx: mpsc::Sender<tungstenite::Message>,
    cancel: tokio_util::sync::CancellationToken,
) {
    let mut interval = tokio::time::interval(WS_PING_PERIOD);
    interval.tick().await; // Skip immediate first tick.

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                let ping = tungstenite::Message::Ping(vec![].into());
                if write_tx.send(ping).await.is_err() {
                    break;
                }
            }
        }
    }

    // WS_PONG_WAIT is used by read_pump's pong deadline.
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    #[test]
    fn ws_error_display() {
        let err = WsError::Timeout;
        assert_eq!(err.to_string(), "request timed out");

        let err = WsError::Closed;
        assert_eq!(err.to_string(), "connection closed");

        let err = WsError::PairingFailed("bad code".into());
        assert_eq!(err.to_string(), "pairing failed: bad code");

        let err = WsError::AgentError {
            code: 401,
            message: "unauthorized".into(),
        };
        assert!(err.to_string().contains("401"));
    }

    #[tokio::test]
    async fn handle_text_routes_response_to_pending() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let on_event: Arc<Mutex<Option<EventCallback>>> = Arc::new(Mutex::new(None));

        let (tx, rx) = oneshot::channel();
        pending.lock().await.insert("req-1".into(), tx);

        let msg = Message::new::<()>("req-1", MessageType::Pong, None).unwrap();
        let json = serde_json::to_string(&msg).unwrap();

        handle_text_message(&json, &pending, &on_event).await;

        let resp = rx.await.unwrap();
        assert_eq!(resp.id, "req-1");
        assert_eq!(resp.msg_type, MessageType::Pong);
        assert!(pending.lock().await.is_empty());
    }

    #[tokio::test]
    async fn handle_text_fires_event_callback() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let on_event: Arc<Mutex<Option<EventCallback>>> =
            Arc::new(Mutex::new(Some(Box::new(move |mt, _msg| {
                received_clone.lock().unwrap().push(mt);
            }))));

        let msg = Message::new::<()>("push-1", MessageType::TelemetryData, None).unwrap();
        let json = serde_json::to_string(&msg).unwrap();

        handle_text_message(&json, &pending, &on_event).await;

        let events = received.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], MessageType::TelemetryData);
    }

    #[tokio::test]
    async fn handle_text_ignores_malformed_json() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let on_event: Arc<Mutex<Option<EventCallback>>> = Arc::new(Mutex::new(None));
        handle_text_message("not valid json {{{", &pending, &on_event).await;
    }

    #[tokio::test]
    async fn handle_text_rejects_oversized_message() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let on_event: Arc<Mutex<Option<EventCallback>>> = Arc::new(Mutex::new(None));

        let huge = "x".repeat(WS_MAX_MESSAGE_SIZE + 1);
        handle_text_message(&huge, &pending, &on_event).await;
    }

    #[tokio::test]
    async fn read_pump_fires_disconnect_on_stream_end() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let on_event: Arc<Mutex<Option<EventCallback>>> = Arc::new(Mutex::new(None));
        let disconnected = Arc::new(std::sync::Mutex::new(false));
        let dc = disconnected.clone();
        let on_disconnect: DisconnectCallback =
            Arc::new(Mutex::new(Some(Box::new(move || {
                *dc.lock().unwrap() = true;
            }))));

        let cancel = tokio_util::sync::CancellationToken::new();
        let (write_tx, _write_rx) = mpsc::channel(16);
        let empty = stream::empty::<Result<tungstenite::Message, tungstenite::Error>>();

        let agent_closed = Arc::new(AtomicBool::new(false));
        read_pump(empty, pending, on_event, on_disconnect, agent_closed, write_tx, cancel).await;

        assert!(*disconnected.lock().unwrap());
    }

    #[tokio::test]
    async fn ping_pump_stops_on_cancel() {
        let (tx, _rx) = mpsc::channel(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        let c = cancel.clone();
        let handle = tokio::spawn(async move {
            ping_pump(tx, c).await;
        });

        cancel.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("should stop")
            .expect("no panic");
    }

    #[tokio::test]
    async fn send_binary_builds_correct_wire_format() {
        // Verify the wire frame format: [4 BE bytes len][header JSON][data].
        let (write_tx, mut write_rx) = mpsc::channel::<tungstenite::Message>(16);
        let pending: Arc<Mutex<HashMap<String, oneshot::Sender<Message>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let on_event: Arc<Mutex<Option<EventCallback>>> = Arc::new(Mutex::new(None));
        let on_disconnect: DisconnectCallback = Arc::new(Mutex::new(None));
        let cancel = tokio_util::sync::CancellationToken::new();

        let client = WsClient {
            write_tx,
            pending: pending.clone(),
            on_event,
            on_disconnect,
            agent_closed: Arc::new(AtomicBool::new(false)),
            _read_handle: tokio::spawn(async {}),
            _write_handle: tokio::spawn(async {}),
            _ping_handle: tokio::spawn(async {}),
            cancel,
        };

        let header = serde_json::json!({"type": "uploadChunk", "uploadId": "u1"});
        let data = b"hello binary";

        // Spawn send_binary — it will timeout waiting for response, but we can check the frame.
        let send_handle = tokio::spawn(async move {
            let _ = client.send_binary(&header, data).await;
        });

        // Read the frame from the write channel.
        let frame_msg = write_rx.recv().await.unwrap();
        let frame = match frame_msg {
            tungstenite::Message::Binary(b) => b.to_vec(),
            other => panic!("expected binary frame, got {other:?}"),
        };

        // Parse the 4-byte header length.
        assert!(frame.len() > 4);
        let header_len =
            ((frame[0] as usize) << 24)
            | ((frame[1] as usize) << 16)
            | ((frame[2] as usize) << 8)
            | (frame[3] as usize);

        // Parse the JSON header.
        let header_json: serde_json::Value =
            serde_json::from_slice(&frame[4..4 + header_len]).unwrap();
        assert_eq!(header_json["type"], "uploadChunk");
        assert_eq!(header_json["uploadId"], "u1");
        // UUID was injected.
        assert!(header_json["id"].is_string());
        assert!(!header_json["id"].as_str().unwrap().is_empty());

        // The binary data follows.
        let binary_data = &frame[4 + header_len..];
        assert_eq!(binary_data, b"hello binary");

        send_handle.abort();
    }

    #[tokio::test]
    async fn read_pump_timeout_on_silence() {
        // With no messages arriving, the pong deadline should fire and
        // trigger a disconnect within WS_PONG_WAIT.
        tokio::time::pause();

        let pending = Arc::new(Mutex::new(HashMap::new()));
        let on_event: Arc<Mutex<Option<EventCallback>>> = Arc::new(Mutex::new(None));
        let disconnected = Arc::new(std::sync::Mutex::new(false));
        let dc = disconnected.clone();
        let on_disconnect: DisconnectCallback =
            Arc::new(Mutex::new(Some(Box::new(move || {
                *dc.lock().unwrap() = true;
            }))));

        let cancel = tokio_util::sync::CancellationToken::new();
        let (write_tx, _write_rx) = mpsc::channel(16);

        // A stream that never yields (pending forever) — simulates silence.
        let stream = stream::pending::<Result<tungstenite::Message, tungstenite::Error>>();
        let agent_closed = Arc::new(AtomicBool::new(false));

        read_pump(stream, pending, on_event, on_disconnect, agent_closed, write_tx, cancel).await;

        assert!(*disconnected.lock().unwrap(), "should disconnect on pong timeout");
    }

    #[tokio::test]
    async fn read_pump_resets_deadline_on_any_message() {
        // Sending a Text message just before the deadline should extend it.
        tokio::time::pause();

        let pending = Arc::new(Mutex::new(HashMap::new()));
        let on_event: Arc<Mutex<Option<EventCallback>>> = Arc::new(Mutex::new(None));
        let disconnected = Arc::new(std::sync::Mutex::new(false));
        let dc = disconnected.clone();
        let on_disconnect: DisconnectCallback =
            Arc::new(Mutex::new(Some(Box::new(move || {
                *dc.lock().unwrap() = true;
            }))));

        let cancel = tokio_util::sync::CancellationToken::new();
        let (write_tx, _write_rx) = mpsc::channel(16);

        // Build a stream that yields one Text message after (PONG_WAIT - 1s),
        // then nothing until the extended deadline expires.
        let wait_before_msg = WS_PONG_WAIT - std::time::Duration::from_secs(1);
        let msg = Message::new::<()>("msg-1", MessageType::Pong, None).unwrap();
        let json = serde_json::to_string(&msg).unwrap();
        let text_msg: Result<tungstenite::Message, tungstenite::Error> =
            Ok(tungstenite::Message::Text(json.into()));

        // Delayed message followed by infinite pending. Box::pin for Unpin.
        let delayed = stream::once(async move {
            tokio::time::sleep(wait_before_msg).await;
            text_msg
        });
        let combined = Box::pin(delayed.chain(stream::pending()));

        let agent_closed = Arc::new(AtomicBool::new(false));
        let handle = tokio::spawn(async move {
            read_pump(combined, pending, on_event, on_disconnect, agent_closed, write_tx, cancel).await;
        });

        // Advance past the original deadline — should NOT have timed out
        // because the message resets the deadline.
        tokio::time::advance(WS_PONG_WAIT + std::time::Duration::from_secs(1)).await;
        // Yield multiple times so the spawned task processes the message and resets.
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert!(!*disconnected.lock().unwrap(), "should not disconnect — deadline was reset");

        // Now advance past the reset deadline (from the message time).
        tokio::time::advance(WS_PONG_WAIT).await;
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        // With paused time the spawned task should have completed.
        handle.await.unwrap();
        assert!(*disconnected.lock().unwrap(), "should disconnect after extended deadline");
    }

    #[tokio::test]
    async fn write_pump_stops_on_cancel() {
        let (sink_tx, mut sink_rx) = mpsc::channel::<tungstenite::Message>(16);
        let cancel = tokio_util::sync::CancellationToken::new();

        let sink =
            futures_util::sink::unfold(sink_tx, |tx, msg: tungstenite::Message| async move {
                let _ = tx.send(msg).await;
                Ok::<_, tungstenite::Error>(tx)
            });
        let sink = Box::pin(sink);

        let (_write_tx, write_rx) = mpsc::channel(16);
        let c = cancel.clone();
        let handle = tokio::spawn(async move {
            write_pump(sink, write_rx, c).await;
        });

        cancel.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("should stop")
            .expect("no panic");

        let close_msg = sink_rx.recv().await;
        assert!(matches!(close_msg, Some(tungstenite::Message::Close(_))));
    }
}
