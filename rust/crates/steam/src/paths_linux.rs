use std::path::PathBuf;

use crate::SteamError;

/// Returns the Steam base directory on Linux/Unix systems.
pub(crate) fn get_base_dir() -> Result<PathBuf, SteamError> {
    let home = home_dir()?;

    // Primary location: ~/.steam/steam
    let steam_dir = home.join(".steam").join("steam");
    if steam_dir.exists() {
        return Ok(steam_dir);
    }

    // Fallback: ~/.local/share/Steam
    let steam_dir = home.join(".local").join("share").join("Steam");
    if steam_dir.exists() {
        return Ok(steam_dir);
    }

    // Flatpak location
    let steam_dir = home
        .join(".var")
        .join("app")
        .join("com.valvesoftware.Steam")
        .join(".steam")
        .join("steam");
    if steam_dir.exists() {
        return Ok(steam_dir);
    }

    Err(SteamError::NotFound)
}

fn home_dir() -> Result<PathBuf, SteamError> {
    dirs_next().ok_or(SteamError::NotFound)
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
