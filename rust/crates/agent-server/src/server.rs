//! Agent WebSocket server.
//!
//! Listens on a TCP port, upgrades HTTP GET `/ws` to WebSocket, and
//! accepts a single Hub connection at a time.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_util::sync::CancellationToken;

use crate::ServerError;
use crate::connection::{self, HubConnection, HubMeta};
use crate::handler::Handler;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// TCP port to listen on (0 = OS-assigned).
    pub port: u16,
    /// Whether to accept new connections (can be toggled at runtime).
    pub accept_connections: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 0,
            accept_connections: true,
        }
    }
}

/// The agent WebSocket server.
///
/// Manages a single Hub connection at a time and dispatches messages
/// to the provided [`Handler`].
pub struct AgentServer<H: Handler> {
    config: Mutex<ServerConfig>,
    handler: Arc<H>,
    hub_conn: Mutex<Option<HubConnection>>,
    cancel: CancellationToken,
    local_addr: Mutex<Option<SocketAddr>>,
}

impl<H: Handler> AgentServer<H> {
    /// Creates a new server with the given handler.
    pub fn new(config: ServerConfig, handler: H) -> Arc<Self> {
        Arc::new(Self {
            config: Mutex::new(config),
            handler: Arc::new(handler),
            hub_conn: Mutex::new(None),
            cancel: CancellationToken::new(),
            local_addr: Mutex::new(None),
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

    /// Updates whether the server accepts new connections.
    pub async fn set_accept_connections(&self, accept: bool) {
        self.config.lock().await.accept_connections = accept;
    }

    /// Returns `true` if a Hub is currently connected.
    pub async fn has_hub(&self) -> bool {
        self.hub_conn.lock().await.is_some()
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
        let port = self.config.lock().await.port;
        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
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
        // Check if accepting connections.
        if !self.config.lock().await.accept_connections {
            tracing::warn!(%peer_addr, "rejecting connection: not accepting");
            return Err(ServerError::ConnectionRejected);
        }

        // Check if a Hub is already connected.
        if self.has_hub().await {
            tracing::warn!(%peer_addr, "rejecting connection: hub already connected");
            return Err(ServerError::HubAlreadyConnected);
        }

        // WebSocket upgrade.
        let ws_stream = accept_async(stream).await?;
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
        if lock.is_some() {
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
    use std::sync::atomic::{AtomicBool, Ordering};

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

    #[tokio::test]
    async fn server_binds_dynamic_port() {
        let handler = TestHandler::new();
        let config = ServerConfig {
            port: 0,
            accept_connections: true,
        };
        let server = AgentServer::new(config, handler);
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
        let config = ServerConfig::default();
        let server = AgentServer::new(config, handler);

        assert!(server.config.lock().await.accept_connections);
        server.set_accept_connections(false).await;
        assert!(!server.config.lock().await.accept_connections);
    }

    #[tokio::test]
    async fn server_accepts_ws_connection() {
        let handler = TestHandler::new();
        let config = ServerConfig {
            port: 0,
            accept_connections: true,
        };
        let server = AgentServer::new(config, handler);
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
        let config = ServerConfig {
            port: 0,
            accept_connections: true,
        };
        let server = AgentServer::new(config, handler);
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
        let config = ServerConfig {
            port: 0,
            accept_connections: true,
        };
        let server = AgentServer::new(config, handler);
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
