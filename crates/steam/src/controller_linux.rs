//! Linux-specific Steam controller implementation.

use crate::SteamError;

/// Checks if Steam is running via `pgrep -x steam`.
pub async fn is_running() -> bool {
    let output = tokio::process::Command::new("pgrep")
        .args(["-x", "steam"])
        .output()
        .await;

    match output {
        Ok(o) => o.status.success() && !o.stdout.is_empty(),
        Err(_) => false,
    }
}

/// Checks if running in gaming mode (gamescope session).
pub fn is_gaming_mode() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v == "gamescope")
        .unwrap_or(false)
}

/// Starts Steam in the background.
pub async fn start() -> Result<(), SteamError> {
    tokio::process::Command::new("steam")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| SteamError::Controller(format!("failed to start Steam: {e}")))?;

    Ok(())
}

/// Sends the graceful shutdown command.
pub async fn shutdown_command() -> Result<(), SteamError> {
    let output = tokio::process::Command::new("steam")
        .arg("-shutdown")
        .output()
        .await
        .map_err(|e| SteamError::Controller(format!("failed to send shutdown: {e}")))?;

    if !output.status.success() {
        tracing::warn!("steam -shutdown returned non-zero, trying fallback");
    }

    Ok(())
}

/// Force-kills the Steam process.
pub async fn force_kill() -> Result<(), SteamError> {
    let output = tokio::process::Command::new("pkill")
        .args(["-9", "steam"])
        .output()
        .await
        .map_err(|e| SteamError::Controller(format!("failed to force kill Steam: {e}")))?;

    if !output.status.success() {
        return Err(SteamError::Controller("pkill -9 steam failed".into()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gaming_mode_detects_env() {
        // Without gamescope env, should be false.
        assert!(!is_gaming_mode());
    }

    #[tokio::test]
    async fn is_running_does_not_panic() {
        let _ = is_running().await;
    }
}
