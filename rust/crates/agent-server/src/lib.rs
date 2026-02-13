//! WebSocket server for the CapyDeploy agent.
//!
//! Accepts a single Hub connection at a time over WebSocket, dispatches
//! JSON and binary messages to a [`Handler`] trait, and manages the
//! connection lifecycle (ping/pong, graceful shutdown).
//!
//! Port of the Go `apps/agents/desktop/wsserver.go` module.

mod binary;
mod connection;
mod handler;
mod server;

pub use binary::{BinaryArtworkHeader, BinaryChunkHeader, BinaryMessage, parse_binary_message};
pub use connection::HubConnection;
pub use handler::Handler;
pub use server::{AgentServer, ServerConfig};

/// Send buffer capacity (matches Go's channel size).
pub const SEND_BUFFER_SIZE: usize = 256;

/// Errors produced by the agent server.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("server already running")]
    AlreadyRunning,

    #[error("hub already connected")]
    HubAlreadyConnected,

    #[error("connection rejected")]
    ConnectionRejected,
}
