//! Hub installed games management: list, delete, artwork update, log wrapper.
//!
//! This crate implements the **business logic** for managing installed games
//! on remote Agents. It is a library crate with no UI or transport
//! dependencies — the Hub app provides an `AgentConnection` implementation
//! that bridges to the actual WebSocket client.
//!
//! # Operations
//!
//! - **List** — get all installed (non-Steam) games via shortcuts
//! - **Delete** — remove a game (agent handles files + shortcut + Steam restart)
//! - **Artwork** — update artwork from local files or remote URLs
//! - **Log wrapper** — enable/disable game log wrapper

pub mod error;
pub mod games;
pub mod types;

// Re-export primary types for convenience.
pub use error::GamesError;
pub use games::{AgentConnection, GamesManager};
pub use types::{ArtworkUpdate, InstalledGame};
