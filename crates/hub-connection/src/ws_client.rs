//! WebSocket client for Hub-to-Agent communication.
//!
//! Implements request-response pattern with UUID correlation,
//! ping/pong keepalive, and push event dispatching.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use futures_util::StreamExt;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::tungstenite;

use capydeploy_protocol::constants::{
    MessageType, WS_BINARY_REQUEST_TIMEOUT, WS_MAX_MESSAGE_SIZE, WS_REQUEST_TIMEOUT,
};
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages::{
    AgentStatusResponse, HubConnectedRequest, PairSuccessResponse, PairingRequiredResponse,
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
pub(crate) type DisconnectCallback = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;

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
        let mut ws_config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
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
            tokio::spawn(crate::pumps::write::write_pump(write, write_rx, cancel))
        };

        let read_handle = {
            let pending = pending.clone();
            let on_event = on_event.clone();
            let on_disconnect = on_disconnect.clone();
            let agent_closed = agent_closed.clone();
            let cancel = cancel.clone();
            let write_tx = write_tx.clone();
            tokio::spawn(crate::pumps::read::read_pump(
                read,
                pending,
                on_event,
                on_disconnect,
                agent_closed,
                write_tx,
                cancel,
            ))
        };

        let ping_handle = {
            let write_tx = write_tx.clone();
            let cancel = cancel.clone();
            tokio::spawn(crate::pumps::ping::ping_pump(write_tx, cancel))
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

        let result = crate::pairing_flow::perform_handshake(&client, hub_request).await?;
        Ok((client, result))
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
        crate::pairing_flow::confirm_pairing(self, code).await
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let header_len = ((frame[0] as usize) << 24)
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
}
