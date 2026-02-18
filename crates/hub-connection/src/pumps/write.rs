//! WebSocket write pump — serialises outbound messages.

use futures_util::SinkExt;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite;
use tokio_util::sync::CancellationToken;
use tracing::error;

/// Writes messages to the WebSocket.
pub(crate) async fn write_pump<S>(
    mut write: S,
    mut write_rx: mpsc::Receiver<tungstenite::Message>,
    cancel: CancellationToken,
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::sink;

    #[tokio::test]
    async fn write_pump_stops_on_cancel() {
        let (sink_tx, mut sink_rx) = mpsc::channel::<tungstenite::Message>(16);
        let cancel = CancellationToken::new();

        let sink =
            sink::unfold(sink_tx, |tx, msg: tungstenite::Message| async move {
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
