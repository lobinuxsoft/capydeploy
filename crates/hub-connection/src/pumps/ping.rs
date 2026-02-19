//! WebSocket ping pump — periodic keepalive pings.

use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite;
use tokio_util::sync::CancellationToken;

use capydeploy_protocol::constants::WS_PING_PERIOD;

/// Sends periodic pings to keep the connection alive.
pub(crate) async fn ping_pump(
    write_tx: mpsc::Sender<tungstenite::Message>,
    cancel: CancellationToken,
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

    #[tokio::test]
    async fn ping_pump_stops_on_cancel() {
        let (tx, _rx) = mpsc::channel(16);
        let cancel = CancellationToken::new();

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
}
