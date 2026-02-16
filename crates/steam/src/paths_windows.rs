use std::path::PathBuf;

use crate::SteamError;

/// Returns the Steam base directory on Windows using the registry.
pub(crate) fn get_base_dir() -> Result<PathBuf, SteamError> {
    // Try 64-bit registry first
    if let Ok(path) = read_steam_registry(r"SOFTWARE\Wow6432Node\Valve\Steam") {
        return Ok(path);
    }

    // Fall back to 32-bit registry
    if let Ok(path) = read_steam_registry(r"SOFTWARE\Valve\Steam") {
        return Ok(path);
    }

    Err(SteamError::NotFound)
}

fn read_steam_registry(subkey: &str) -> Result<PathBuf, SteamError> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(subkey).map_err(|_| SteamError::NotFound)?;
    let install_path: String = key
        .get_value("InstallPath")
        .map_err(|_| SteamError::NotFound)?;
    Ok(PathBuf::from(install_path))
}
