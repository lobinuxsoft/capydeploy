//! Error types for the data channel.

/// Errors produced by the TCP data channel.
#[derive(Debug, thiserror::Error)]
pub enum DataChannelError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("invalid file path: {0}")]
    InvalidPath(String),

    #[error("connection timed out")]
    Timeout,

    #[error("cancelled")]
    Cancelled,

    #[error("protocol error: {0}")]
    Protocol(String),
}
