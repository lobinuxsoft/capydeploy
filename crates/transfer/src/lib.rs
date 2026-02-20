//! Chunked file transfer with resume support and progress tracking.
//!
//! Port of `pkg/transfer/` from the Go codebase.

mod chunked;
mod progress;
mod types;
mod validation;

pub use chunked::{
    ChecksumError, ChunkReader, ChunkWriter, calculate_file_checksum, checksum_bytes,
};
pub use progress::{ProgressTracker, SpeedCalculator};
pub use types::{Chunk, UploadSession};
pub use validation::validate_upload_path;

/// Default chunk size: 4 MiB.
///
/// Larger chunks reduce per-chunk overhead (SHA-256, ACKs, syscalls).
/// The agent negotiates the actual size via `InitUploadResponseFull.chunk_size`.
pub const DEFAULT_CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// Errors produced by the transfer crate.
#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("checksum mismatch")]
    ChecksumMismatch,

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("session not active: {0}")]
    SessionNotActive(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),
}
