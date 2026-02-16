//! Error types for games management operations.

/// Errors produced during installed games management.
#[derive(Debug, thiserror::Error)]
pub enum GamesError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("agent error: {0}")]
    Agent(String),

    #[error("no agent connected")]
    NoAgent,

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("artwork error: {0}")]
    Artwork(String),
}
