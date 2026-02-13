//! Hub connection management: read/write pumps, ping/pong, send buffering.

use std::sync::Arc;

use capydeploy_protocol::constants::{
    MessageType, WS_MAX_MESSAGE_SIZE, WS_PING_PERIOD, WS_PONG_WAIT,
};
use capydeploy_protocol::envelope::Message;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_util::sync::CancellationToken;

use crate::SEND_BUFFER_SIZE;
use crate::binary::{BinaryMessage, parse_binary_message};
use crate::handler::Handler;

/// Metadata about the connected Hub.
#[derive(Debug, Clone)]
pub struct HubMeta {
    pub name: String,
    pub version: String,
    pub hub_id: String,
    pub remote_addr: String,
}

/// Handle for sending messages to the connected Hub.
///
/// Cloneable and cheap — wraps an `mpsc::Sender`.
#[derive(Clone)]
pub struct Sender {
    tx: mpsc::Sender<WsMessage>,
}

impl Sender {
    /// Sends a protocol [`Message`] as JSON text.
    ///
    /// Returns `Err` only if the channel is closed (Hub disconnected).
    pub fn send_msg(&self, msg: Message) -> Result<(), SendError> {
        let json = serde_json::to_string(&msg).map_err(|_| SendError)?;
        self.tx.try_send(WsMessage::Text(json.into())).map_err(|_| {
            tracing::warn!("send buffer full or closed, dropping message");
            SendError
        })
    }

    /// Sends an error response for the given request message.
    pub fn send_error(&self, req: &Message, code: i32, message: &str) -> Result<(), SendError> {
        self.send_msg(req.reply_error(code, message))
    }

    /// Sends a raw binary frame.
    pub fn send_binary(&self, data: Vec<u8>) -> Result<(), SendError> {
        self.tx
            .try_send(WsMessage::Binary(data.into()))
            .map_err(|_| SendError)
    }

    /// Returns `true` if the send channel is still open.
    pub fn is_connected(&self) -> bool {
        !self.tx.is_closed()
    }
}

/// Error returned when the send channel is full or closed.
#[derive(Debug, thiserror::Error)]
#[error("send failed: buffer full or connection closed")]
pub struct SendError;

/// Active connection to a Hub.
///
/// Owns the read/write pump tasks and provides a [`Sender`] for
/// asynchronous message delivery.
pub struct HubConnection {
    pub meta: HubMeta,
    sender: Sender,
    cancel: CancellationToken,
}

impl HubConnection {
    /// Returns a cloneable [`Sender`] for this connection.
    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }

    /// Signals shutdown and waits for the pumps to finish.
    pub fn close(&self) {
        self.cancel.cancel();
    }
}

/// Runs the read and write pumps for a WebSocket connection.
///
/// Returns the [`HubConnection`] handle. The pumps run as background
/// tokio tasks and stop when the connection is closed or the cancel
/// token is triggered.
pub fn spawn_connection<S, H>(
    ws_stream: S,
    meta: HubMeta,
    handler: Arc<H>,
    server_cancel: CancellationToken,
) -> HubConnection
where
    S: futures_util::Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
        + futures_util::Sink<WsMessage, Error = tokio_tungstenite::tungstenite::Error>
        + Send
        + 'static,
    H: Handler,
{
    let (tx, rx) = mpsc::channel::<WsMessage>(SEND_BUFFER_SIZE);
    let cancel = server_cancel.child_token();
    let sender = Sender { tx };

    let conn = HubConnection {
        meta: meta.clone(),
        sender: sender.clone(),
        cancel: cancel.clone(),
    };

    let (ws_sink, ws_stream) = ws_stream.split();

    // Write pump.
    let write_cancel = cancel.clone();
    tokio::spawn(write_pump(ws_sink, rx, write_cancel));

    // Read pump.
    let read_cancel = cancel.clone();
    let read_handler = handler.clone();
    let read_sender = sender.clone();
    tokio::spawn(async move {
        read_pump(ws_stream, read_sender, read_handler, read_cancel.clone()).await;
        // When read pump exits, cancel the write pump too.
        read_cancel.cancel();
        handler.on_hub_disconnected().await;
        tracing::info!(hub = %meta.name, "hub disconnected");
    });

    conn
}

/// Write pump: drains the send channel and sends WS pings.
async fn write_pump<S>(mut sink: S, mut rx: mpsc::Receiver<WsMessage>, cancel: CancellationToken)
where
    S: futures_util::Sink<WsMessage, Error = tokio_tungstenite::tungstenite::Error> + Send + Unpin,
{
    let mut ping_interval = tokio::time::interval(WS_PING_PERIOD);
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,

            msg = rx.recv() => {
                match msg {
                    Some(ws_msg) => {
                        if let Err(e) = sink.send(ws_msg).await {
                            tracing::error!("write pump send error: {e}");
                            break;
                        }
                    }
                    None => break, // Channel closed.
                }
            }

            _ = ping_interval.tick() => {
                if let Err(e) = sink.send(WsMessage::Ping(Vec::new().into())).await {
                    tracing::error!("write pump ping error: {e}");
                    break;
                }
            }
        }
    }

    // Best-effort close frame.
    let _ = sink.close().await;
}

/// Read pump: reads WS frames and dispatches to the handler.
async fn read_pump<S, H>(mut stream: S, sender: Sender, handler: Arc<H>, cancel: CancellationToken)
where
    S: futures_util::Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
        + Send
        + Unpin,
    H: Handler,
{
    let mut pong_deadline = tokio::time::interval(WS_PONG_WAIT);
    pong_deadline.reset();
    let mut got_pong = true;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,

            _ = pong_deadline.tick() => {
                if !got_pong {
                    tracing::warn!("pong timeout, closing connection");
                    break;
                }
                got_pong = false;
            }

            frame = stream.next() => {
                match frame {
                    Some(Ok(ws_msg)) => {
                        match ws_msg {
                            WsMessage::Text(text) => {
                                if text.len() > WS_MAX_MESSAGE_SIZE {
                                    tracing::error!("message exceeds max size ({} > {})", text.len(), WS_MAX_MESSAGE_SIZE);
                                    continue;
                                }
                                dispatch_text(&handler, &sender, &text).await;
                            }
                            WsMessage::Binary(data) => {
                                if data.len() > WS_MAX_MESSAGE_SIZE {
                                    tracing::error!("binary message exceeds max size ({} > {})", data.len(), WS_MAX_MESSAGE_SIZE);
                                    continue;
                                }
                                dispatch_binary(&handler, &sender, &data).await;
                            }
                            WsMessage::Pong(_) => {
                                got_pong = true;
                                pong_deadline.reset();
                            }
                            WsMessage::Ping(data) => {
                                // Auto-respond to pings from Hub.
                                let _ = sender.tx.try_send(WsMessage::Pong(data));
                            }
                            WsMessage::Close(_) => {
                                tracing::info!("received close frame");
                                break;
                            }
                            WsMessage::Frame(_) => {} // Raw frames ignored.
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("read pump error: {e}");
                        break;
                    }
                    None => break, // Stream ended.
                }
            }
        }
    }
}

/// Dispatches a text (JSON) message to the appropriate handler method.
async fn dispatch_text<H: Handler>(handler: &Arc<H>, sender: &Sender, text: &str) {
    let msg: Message = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("invalid message JSON: {e}");
            return;
        }
    };

    let s = sender.clone();
    match msg.msg_type {
        MessageType::HubConnected => handler.on_hub_connected(s, msg).await,
        MessageType::PairConfirm => handler.on_pair_confirm(s, msg).await,
        MessageType::Ping => handler.on_ping(s, msg).await,
        MessageType::GetInfo => handler.on_get_info(s, msg).await,
        MessageType::GetConfig => handler.on_get_config(s, msg).await,
        MessageType::GetSteamUsers => handler.on_get_steam_users(s, msg).await,
        MessageType::ListShortcuts => handler.on_list_shortcuts(s, msg).await,
        MessageType::CreateShortcut => handler.on_create_shortcut(s, msg).await,
        MessageType::DeleteShortcut => handler.on_delete_shortcut(s, msg).await,
        MessageType::DeleteGame => handler.on_delete_game(s, msg).await,
        MessageType::ApplyArtwork => handler.on_apply_artwork(s, msg).await,
        MessageType::RestartSteam => handler.on_restart_steam(s, msg).await,
        MessageType::InitUpload => handler.on_init_upload(s, msg).await,
        MessageType::UploadChunk => handler.on_upload_chunk(s, msg).await,
        MessageType::CompleteUpload => handler.on_complete_upload(s, msg).await,
        MessageType::CancelUpload => handler.on_cancel_upload(s, msg).await,
        MessageType::SetConsoleLogFilter => handler.on_set_console_log_filter(s, msg).await,
        MessageType::SetConsoleLogEnabled => handler.on_set_console_log_enabled(s, msg).await,
        MessageType::SetGameLogWrapper => handler.on_set_game_log_wrapper(s, msg).await,
        _ => {
            tracing::warn!(msg_type = ?msg.msg_type, "unhandled message type");
            let _ = sender.send_error(&msg, 501, "unknown message type");
        }
    }
}

/// Dispatches a binary WebSocket frame to the appropriate handler method.
async fn dispatch_binary<H: Handler>(handler: &Arc<H>, sender: &Sender, data: &[u8]) {
    match parse_binary_message(data) {
        Ok(BinaryMessage::Chunk { header, data }) => {
            handler.on_binary_chunk(sender.clone(), header, data).await;
        }
        Ok(BinaryMessage::Artwork { header, data }) => {
            handler
                .on_binary_artwork(sender.clone(), header, data)
                .await;
        }
        Err(e) => {
            tracing::error!("failed to parse binary message: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sender_error_display() {
        let err = SendError;
        assert!(err.to_string().contains("buffer full"));
    }

    #[test]
    fn hub_meta_clone() {
        let meta = HubMeta {
            name: "TestHub".into(),
            version: "1.0".into(),
            hub_id: "h1".into(),
            remote_addr: "127.0.0.1".into(),
        };
        let cloned = meta.clone();
        assert_eq!(meta.name, cloned.name);
    }
}
