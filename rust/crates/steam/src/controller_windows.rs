//! Windows-specific Steam controller implementation.

use std::path::PathBuf;

use crate::SteamError;

/// Known Steam install paths on Windows.
const STEAM_PATHS: &[&str] = &[
    r"C:\Program Files (x86)\Steam\steam.exe",
    r"C:\Program Files\Steam\steam.exe",
];

/// Checks if Steam is running via `tasklist`.
pub async fn is_running() -> bool {
    let output = tokio::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq steam.exe", "/NH"])
        .output()
        .await;

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_lowercase();
            stdout.contains("steam.exe")
        }
        Err(_) => false,
    }
}

/// Windows has no gaming mode.
pub fn is_gaming_mode() -> bool {
    false
}

/// Finds the Steam executable path.
fn find_steam_exe() -> Option<PathBuf> {
    for path in STEAM_PATHS {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Starts Steam.
pub async fn start() -> Result<(), SteamError> {
    if let Some(exe) = find_steam_exe() {
        tokio::process::Command::new(&exe)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| SteamError::Controller(format!("failed to start Steam: {e}")))?;
    } else {
        // Fallback: use URI protocol.
        tokio::process::Command::new("cmd")
            .args(["/C", "start", "steam://open/main"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| SteamError::Controller(format!("failed to start Steam via URI: {e}")))?;
    }

    Ok(())
}

/// Sends the graceful shutdown command.
pub async fn shutdown_command() -> Result<(), SteamError> {
    if let Some(exe) = find_steam_exe() {
        let output = tokio::process::Command::new(&exe)
            .arg("-shutdown")
            .output()
            .await
            .map_err(|e| SteamError::Controller(format!("failed to send shutdown: {e}")))?;

        if !output.status.success() {
            tracing::warn!("steam.exe -shutdown returned non-zero");
        }
    } else {
        // Fallback: URI protocol exit.
        tokio::process::Command::new("cmd")
            .args(["/C", "start", "steam://exit"])
            .output()
            .await
            .map_err(|e| SteamError::Controller(format!("failed to shutdown via URI: {e}")))?;
    }

    Ok(())
}

/// Force-kills the Steam process.
pub async fn force_kill() -> Result<(), SteamError> {
    let output = tokio::process::Command::new("taskkill")
        .args(["/F", "/IM", "steam.exe"])
        .output()
        .await
        .map_err(|e| SteamError::Controller(format!("failed to force kill Steam: {e}")))?;

    if !output.status.success() {
        return Err(SteamError::Controller(
            "taskkill /F /IM steam.exe failed".into(),
        ));
    }

    Ok(())
}
