pub mod controller;
pub mod paths;
#[cfg(target_os = "linux")]
pub mod paths_linux;
#[cfg(target_os = "windows")]
pub mod paths_windows;
pub mod shortcuts;
pub mod users;
pub mod vdf;

// Re-export primary types.
pub use controller::Controller;
pub use paths::{ArtworkType, Paths};
pub use shortcuts::{ShortcutManager, convert_to_shortcut_info, generate_app_id};
pub use users::{User, get_users, get_users_with_paths, u32_to_user_id, user_id_to_u32};
pub use vdf::load_shortcuts_vdf;

/// Errors for Steam operations.
#[derive(Debug, thiserror::Error)]
pub enum SteamError {
    #[error("steam installation not found")]
    NotFound,

    #[error("steam user not found")]
    UserNotFound,

    #[error("image not found")]
    ImageNotFound,

    #[error("shortcuts.vdf not found")]
    ShortcutsNotFound,

    #[error("VDF parse error: {0}")]
    Vdf(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("controller error: {0}")]
    Controller(String),

    #[error("timeout: {0}")]
    Timeout(String),
}
