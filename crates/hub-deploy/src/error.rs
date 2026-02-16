//! Deploy error types.

/// Errors produced during game deployment.
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("agent error: {0}")]
    Agent(String),

    #[error("no agent connected")]
    NoAgent,

    #[error("upload failed: {0}")]
    Upload(String),

    #[error("artwork error: {0}")]
    Artwork(String),

    #[error("cancelled")]
    Cancelled,

    #[error("SteamGridDB error: {0}")]
    SteamGridDb(#[from] capydeploy_steamgriddb::client::Error),

    #[error("transfer error: {0}")]
    Transfer(#[from] capydeploy_transfer::TransferError),
}
