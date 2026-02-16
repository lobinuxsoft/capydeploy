//! File operations for game installation, deletion and filesystem browsing.
//!
//! Provides safe path resolution, directory management, permission handling,
//! and filesystem browsing for the agent.

mod browse;
mod delete;
mod install;

pub use browse::{DirEntry, list_directory, platform_roots};
pub use delete::{delete_artwork, delete_game_directory, grid_dir};
pub use install::{ensure_install_dir, resolve_install_path, set_executable};

/// Default game installation directory name under `$HOME`.
pub const DEFAULT_GAMES_DIR: &str = "Games";

/// Resolves the default install base path.
///
/// Returns `$HOME/Games` or `/tmp/Games` as fallback.
pub fn default_install_path() -> std::path::PathBuf {
    let home = home_dir();
    home.join(DEFAULT_GAMES_DIR)
}

/// Returns the user's home directory.
fn home_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
}
