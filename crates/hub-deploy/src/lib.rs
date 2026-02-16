//! Hub game deploy flow: scan, upload, artwork, shortcut creation.
//!
//! This crate implements the **business logic** for deploying games
//! from the Hub to remote Agents. It is a library crate with no UI
//! or transport dependencies — the Hub app provides an `AgentConnection`
//! implementation that bridges to the actual WebSocket client.
//!
//! # Pipeline
//!
//! 1. **Scan** — recursively walk the game directory
//! 2. **Init** — negotiate upload session with the Agent
//! 3. **Upload** — send file chunks with resume support
//! 4. **Artwork** — send local images via binary messages
//! 5. **Complete** — finalize upload and create Steam shortcut

pub mod agent;
pub mod artwork_selector;
pub mod deploy;
pub mod error;
pub mod scanner;
pub mod types;

// Re-export primary types for convenience.
pub use agent::AgentConnection;
pub use artwork_selector::{
    ArtworkSelector, build_artwork_assignment, build_remote_artwork_config,
    build_shortcut_config, classify_artwork_source, collect_local_artwork,
    detect_content_type, parse_tags,
};
pub use deploy::DeployOrchestrator;
pub use error::DeployError;
pub use scanner::scan_files_for_upload;
pub use types::{
    ArtworkAssignment, ArtworkSource, CompleteUploadResult, DeployConfig, DeployEvent,
    DeployResult, GameSetup, InitUploadResult, LocalArtwork,
};
