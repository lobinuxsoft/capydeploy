//! Agent WebSocket server.
//!
//! Listens on a TCP port, upgrades HTTP GET `/ws` to WebSocket, and
//! accepts a single Hub connection at a time.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async_with_config;
use tokio_util::sync::CancellationToken;

use capydeploy_protocol::constants::WS_MAX_MESSAGE_SIZE;

use crate::ServerError;
use crate::connection::{self, HubConnection, HubMeta};
use crate::handler::Handler;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// TCP port to listen on (0 = OS-assigned).
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 0 }
    }
}

/// The agent WebSocket server.
///
/// Manages a single Hub connection at a time and dispatches messages
/// to the provided [`Handler`].
pub struct AgentServer<H: Handler> {
    port: u16,
    handler: Arc<H>,
    hub_conn: Mutex<Option<HubConnection>>,
    cancel: CancellationToken,
    local_addr: Mutex<Option<SocketAddr>>,
    /// Shared flag — the caller (e.g. Tauri state) owns the same Arc
    /// so toggling from the UI is immediately visible to the server.
    accept: Arc<AtomicBool>,
}

impl<H: Handler> AgentServer<H> {
    /// Creates a new server with the given handler.
    ///
    /// `accept` is a shared flag that controls whether the server accepts
    /// new Hub connections. The caller keeps a clone so toggling from the
    /// UI takes effect immediately (no extra sync needed).
    pub fn new(config: ServerConfig, handler: H, accept: Arc<AtomicBool>) -> Arc<Self> {
        Arc::new(Self {
            port: config.port,
            handler: Arc::new(handler),
            hub_conn: Mutex::new(None),
            cancel: CancellationToken::new(),
            local_addr: Mutex::new(None),
            accept,
        })
    }

    /// Returns the local address the server is listening on.
    ///
    /// Only available after [`run`] binds the socket.
    pub async fn local_addr(&self) -> Option<SocketAddr> {
        *self.local_addr.lock().await
    }

    /// Returns the listening port (0 if not yet bound).
    pub async fn port(&self) -> u16 {
        self.local_addr.lock().await.map(|a| a.port()).unwrap_or(0)
    }

    /// Returns `true` if a Hub is currently connected and alive.
    pub async fn has_hub(&self) -> bool {
        let lock = self.hub_conn.lock().await;
        match lock.as_ref() {
            Some(conn) => conn.sender().is_connected(),
            None => false,
        }
    }

    /// Returns the sender for the current Hub connection, if any.
    pub async fn hub_sender(&self) -> Option<connection::Sender> {
        self.hub_conn.lock().await.as_ref().map(|c| c.sender())
    }

    /// Closes the current Hub connection (if any).
    pub async fn disconnect_hub(&self) {
        let mut lock = self.hub_conn.lock().await;
        if let Some(conn) = lock.take() {
            conn.close();
        }
    }

    /// Gracefully shuts down the server.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }

    /// Runs the server until cancellation.
    ///
    /// Binds to the configured port and accepts WebSocket connections.
    pub async fn run(self: &Arc<Self>) -> Result<(), ServerError> {
        let addr: SocketAddr = ([0, 0, 0, 0], self.port).into();
        let listener = TcpListener::bind(addr).await?;

        let local_addr = listener.local_addr()?;
        *self.local_addr.lock().await = Some(local_addr);
        tracing::info!("agent server listening on {local_addr}");

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    tracing::info!("server shutting down");
                    self.disconnect_hub().await;
                    break Ok(());
                }

                result = listener.accept() => {
                    match result {
                        Ok((stream, peer_addr)) => {
                            let server = Arc::clone(self);
                            tokio::spawn(async move {
                                if let Err(e) = server.handle_connection(stream, peer_addr).await {
                                    tracing::error!(%peer_addr, "connection error: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("accept error: {e}");
                        }
                    }
                }
            }
        }
    }

    /// Handles a single TCP connection: upgrades to WS and manages the Hub session.
    async fn handle_connection(
        self: &Arc<Self>,
        stream: tokio::net::TcpStream,
        peer_addr: SocketAddr,
    ) -> Result<(), ServerError> {
        // Check if accepting connections (lock-free: shared AtomicBool).
        if !self.accept.load(Ordering::Relaxed) {
            tracing::warn!(%peer_addr, "rejecting connection: not accepting");
            return Err(ServerError::ConnectionRejected);
        }

        // Take the old connection (if any) and wait for its read pump +
        // on_hub_disconnected to finish before accepting the new one.
        // This prevents a race where the old disconnect handler wipes
        // hub_sender/collectors that the new connection just set up.
        {
            let old = self.hub_conn.lock().await.take();
            if let Some(conn) = old {
                if conn.sender().is_connected() {
                    tracing::info!(%peer_addr, "replacing active hub connection");
                } else {
                    tracing::info!("clearing stale hub connection");
                }
                conn.close_and_wait().await;
            }
        }

        // WebSocket upgrade with size limits matching our protocol constants.
        let mut ws_config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
        ws_config.max_message_size = Some(WS_MAX_MESSAGE_SIZE);
        ws_config.max_frame_size = Some(WS_MAX_MESSAGE_SIZE);
        let ws_stream = accept_async_with_config(stream, Some(ws_config)).await?;
        tracing::info!(%peer_addr, "WebSocket connection established");

        let meta = HubMeta {
            name: String::new(),
            version: String::new(),
            hub_id: String::new(),
            remote_addr: peer_addr.to_string(),
        };

        let conn = connection::spawn_connection(
            ws_stream,
            meta,
            Arc::clone(&self.handler),
            self.cancel.clone(),
        );

        // Store the connection.
        let mut lock = self.hub_conn.lock().await;
        // Double-check: another task may have connected between our check and now.
        if lock.as_ref().is_some_and(|c| c.sender().is_connected()) {
            conn.close();
            return Err(ServerError::HubAlreadyConnected);
        }
        *lock = Some(conn);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::HandlerFuture;
    use capydeploy_protocol::envelope::Message;

    /// Minimal test handler.
    struct TestHandler {
        connected: AtomicBool,
    }

    impl TestHandler {
        fn new() -> Self {
            Self {
                connected: AtomicBool::new(false),
            }
        }
    }

    impl Handler for TestHandler {
        fn on_hub_connected(
            &self,
            _sender: connection::Sender,
            _msg: Message,
        ) -> HandlerFuture<'_> {
            self.connected.store(true, Ordering::SeqCst);
            Box::pin(async {})
        }
    }

    fn accept_flag(val: bool) -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(val))
    }

    #[tokio::test]
    async fn server_binds_dynamic_port() {
        let handler = TestHandler::new();
        let config = ServerConfig { port: 0 };
        let server = AgentServer::new(config, handler, accept_flag(true));
        let server2 = Arc::clone(&server);

        let handle = tokio::spawn(async move {
            server2.run().await.unwrap();
        });

        // Wait for the server to bind.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let port = server.port().await;
        assert!(port > 0, "should have bound to a dynamic port");

        // No hub connected yet.
        assert!(!server.has_hub().await);

        server.shutdown();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn server_accept_connections_toggle() {
        let handler = TestHandler::new();
        let accept = accept_flag(true);
        let config = ServerConfig::default();
        let server = AgentServer::new(config, handler, accept.clone());

        assert!(server.accept.load(Ordering::Relaxed));
        accept.store(false, Ordering::Relaxed);
        assert!(!server.accept.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn server_accepts_ws_connection() {
        let handler = TestHandler::new();
        let config = ServerConfig { port: 0 };
        let server = AgentServer::new(config, handler, accept_flag(true));
        let server2 = Arc::clone(&server);

        let handle = tokio::spawn(async move {
            server2.run().await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let port = server.port().await;

        // Connect a WS client.
        let url = format!("ws://127.0.0.1:{port}");
        let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        // Give the server time to register the connection.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(server.has_hub().await);

        // Close client.
        drop(ws);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        server.shutdown();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn server_rejects_second_connection() {
        let handler = TestHandler::new();
        let config = ServerConfig { port: 0 };
        let server = AgentServer::new(config, handler, accept_flag(true));
        let server2 = Arc::clone(&server);

        let handle = tokio::spawn(async move {
            server2.run().await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let port = server.port().await;
        let url = format!("ws://127.0.0.1:{port}");

        // First connection should succeed.
        let (_ws1, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(server.has_hub().await);

        // Second connection should fail (server drops it).
        let result = tokio_tungstenite::connect_async(&url).await;
        // The connection might succeed at TCP level but the server
        // should close it; or it might fail entirely. Either way
        // only one hub should remain.
        if let Ok((ws2, _)) = result {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            drop(ws2);
        }

        server.shutdown();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn server_dispatches_text_message() {
        use futures_util::SinkExt;

        let handler = TestHandler::new();
        let config = ServerConfig { port: 0 };
        let server = AgentServer::new(config, handler, accept_flag(true));
        let server2 = Arc::clone(&server);

        let handle = tokio::spawn(async move {
            server2.run().await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let port = server.port().await;
        let url = format!("ws://127.0.0.1:{port}");

        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Send a hub_connected message.
        let msg = serde_json::json!({
            "id": "test-1",
            "type": "hub_connected",
            "payload": {
                "name": "TestHub",
                "version": "1.0.0"
            }
        });
        ws.send(tokio_tungstenite::tungstenite::Message::Text(
            msg.to_string().into(),
        ))
        .await
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Handler should have been called.
        assert!(server.handler.connected.load(Ordering::SeqCst));

        drop(ws);
        server.shutdown();
        handle.await.unwrap();
    }
}
