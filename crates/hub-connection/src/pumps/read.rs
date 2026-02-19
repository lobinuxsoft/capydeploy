//! WebSocket read pump — dispatches incoming messages.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::StreamExt;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::tungstenite;
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace, warn};

use capydeploy_protocol::constants::{WS_MAX_MESSAGE_SIZE, WS_PONG_WAIT};
use capydeploy_protocol::envelope::Message;

use crate::ws_client::{DisconnectCallback, EventCallback};

/// Reads messages from the WebSocket and dispatches them.
///
/// Uses a pong deadline to detect dead connections: if no pong arrives
/// within [`WS_PONG_WAIT`] after a ping was sent, the connection is
/// considered dead and the loop exits (triggering reconnect).
pub(crate) async fn read_pump<S>(
    mut read: S,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Message>>>>,
    on_event: Arc<Mutex<Option<EventCallback>>>,
    on_disconnect: DisconnectCallback,
    agent_closed: Arc<AtomicBool>,
    write_tx: mpsc::Sender<tungstenite::Message>,
    cancel: CancellationToken,
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

#[cfg(test)]
mod tests {
    use super::*;
    use capydeploy_protocol::constants::MessageType;
    use futures_util::stream;

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
        let on_disconnect: DisconnectCallback = Arc::new(Mutex::new(Some(Box::new(move || {
            *dc.lock().unwrap() = true;
        }))));

        let cancel = CancellationToken::new();
        let (write_tx, _write_rx) = mpsc::channel(16);
        let empty = stream::empty::<Result<tungstenite::Message, tungstenite::Error>>();

        let agent_closed = Arc::new(AtomicBool::new(false));
        read_pump(
            empty,
            pending,
            on_event,
            on_disconnect,
            agent_closed,
            write_tx,
            cancel,
        )
        .await;

        assert!(*disconnected.lock().unwrap());
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
        let on_disconnect: DisconnectCallback = Arc::new(Mutex::new(Some(Box::new(move || {
            *dc.lock().unwrap() = true;
        }))));

        let cancel = CancellationToken::new();
        let (write_tx, _write_rx) = mpsc::channel(16);

        // A stream that never yields (pending forever) — simulates silence.
        let stream = stream::pending::<Result<tungstenite::Message, tungstenite::Error>>();
        let agent_closed = Arc::new(AtomicBool::new(false));

        read_pump(
            stream,
            pending,
            on_event,
            on_disconnect,
            agent_closed,
            write_tx,
            cancel,
        )
        .await;

        assert!(
            *disconnected.lock().unwrap(),
            "should disconnect on pong timeout"
        );
    }

    #[tokio::test]
    async fn read_pump_resets_deadline_on_any_message() {
        // Sending a Text message just before the deadline should extend it.
        tokio::time::pause();

        let pending = Arc::new(Mutex::new(HashMap::new()));
        let on_event: Arc<Mutex<Option<EventCallback>>> = Arc::new(Mutex::new(None));
        let disconnected = Arc::new(std::sync::Mutex::new(false));
        let dc = disconnected.clone();
        let on_disconnect: DisconnectCallback = Arc::new(Mutex::new(Some(Box::new(move || {
            *dc.lock().unwrap() = true;
        }))));

        let cancel = CancellationToken::new();
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
            read_pump(
                combined,
                pending,
                on_event,
                on_disconnect,
                agent_closed,
                write_tx,
                cancel,
            )
            .await;
        });

        // Advance past the original deadline — should NOT have timed out
        // because the message resets the deadline.
        tokio::time::advance(WS_PONG_WAIT + std::time::Duration::from_secs(1)).await;
        // Yield multiple times so the spawned task processes the message and resets.
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert!(
            !*disconnected.lock().unwrap(),
            "should not disconnect — deadline was reset"
        );

        // Now advance past the reset deadline (from the message time).
        tokio::time::advance(WS_PONG_WAIT).await;
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        // With paused time the spawned task should have completed.
        handle.await.unwrap();
        assert!(
            *disconnected.lock().unwrap(),
            "should disconnect after extended deadline"
        );
    }
}
