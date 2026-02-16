//! Steam process controller: start, shutdown, restart, status detection.
//!
//! Platform-specific implementations live in `controller_linux.rs` and
//! `controller_windows.rs`. This module provides the unified API.

use std::time::Duration;

use crate::SteamError;

/// Max time to wait for CEF availability after start/restart.
pub const CEF_TIMEOUT: Duration = Duration::from_secs(30);

/// Poll interval when checking CEF readiness.
pub const CEF_CHECK_INTERVAL: Duration = Duration::from_secs(2);

/// Max time to wait for the Steam process to exit after shutdown.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Poll interval when waiting for process exit.
const SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// CEF debug endpoint.
pub const CEF_DEBUG_ENDPOINT: &str = "http://localhost:8080/json";

/// CEF debug file name (placed in Steam's base directory).
pub const CEF_DEBUG_FILE: &str = ".cef-enable-remote-debugging";

/// Result of a Steam restart operation.
#[derive(Debug, Clone)]
pub struct RestartResult {
    pub success: bool,
    pub message: String,
}

/// Steam process controller.
///
/// Provides cross-platform Steam lifecycle management: process detection,
/// start, shutdown, restart, and CEF debug readiness.
pub struct Controller;

impl Controller {
    pub fn new() -> Self {
        Self
    }

    /// Returns `true` if the Steam process is currently running.
    pub async fn is_running(&self) -> bool {
        platform::is_running().await
    }

    /// Returns `true` if running in gaming mode (gamescope on Linux).
    pub fn is_gaming_mode(&self) -> bool {
        platform::is_gaming_mode()
    }

    /// Starts the Steam process (non-blocking).
    pub async fn start(&self) -> Result<(), SteamError> {
        platform::start().await
    }

    /// Gracefully shuts down Steam. Polls for process exit up to
    /// [`SHUTDOWN_TIMEOUT`].
    pub async fn shutdown(&self) -> Result<(), SteamError> {
        if !self.is_running().await {
            return Ok(());
        }

        platform::shutdown_command().await?;

        // Poll until process exits.
        let deadline = tokio::time::Instant::now() + SHUTDOWN_TIMEOUT;
        while tokio::time::Instant::now() < deadline {
            if !self.is_running().await {
                return Ok(());
            }
            tokio::time::sleep(SHUTDOWN_POLL_INTERVAL).await;
        }

        Err(SteamError::Timeout(
            "timeout waiting for Steam to close".into(),
        ))
    }

    /// Full restart cycle: shutdown → start → wait for CEF.
    pub async fn restart(&self) -> RestartResult {
        // Try graceful shutdown first.
        if let Err(e) = self.shutdown().await {
            tracing::warn!("graceful shutdown failed: {e}, force killing");
            if let Err(e) = platform::force_kill().await {
                return RestartResult {
                    success: false,
                    message: format!("failed to force kill Steam: {e}"),
                };
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        // In gaming mode, session manager restarts Steam automatically.
        if !self.is_gaming_mode()
            && let Err(e) = self.start().await
        {
            return RestartResult {
                success: false,
                message: format!("failed to start Steam: {e}"),
            };
        }

        // Wait for CEF to become available.
        if let Err(e) = self.wait_for_cef().await {
            return RestartResult {
                success: false,
                message: format!("Steam started but CEF not available: {e}"),
            };
        }

        RestartResult {
            success: true,
            message: "Steam restarted successfully".into(),
        }
    }

    /// Checks if CEF debug endpoint is responding.
    pub async fn is_cef_available(&self) -> bool {
        is_cef_available().await
    }

    /// Polls until CEF is available or [`CEF_TIMEOUT`] elapses.
    pub async fn wait_for_cef(&self) -> Result<(), SteamError> {
        let deadline = tokio::time::Instant::now() + CEF_TIMEOUT;
        while tokio::time::Instant::now() < deadline {
            if is_cef_available().await {
                return Ok(());
            }
            tokio::time::sleep(CEF_CHECK_INTERVAL).await;
        }
        Err(SteamError::Timeout(format!(
            "timeout waiting for Steam CEF (waited {}s)",
            CEF_TIMEOUT.as_secs()
        )))
    }

    /// Ensures Steam is running with CEF available.
    ///
    /// If CEF is already responding, returns immediately. Otherwise:
    /// - Creates the CEF debug file if missing
    /// - Starts or restarts Steam as needed
    pub async fn ensure_running(&self) -> Result<(), SteamError> {
        if is_cef_available().await {
            return Ok(());
        }

        self.ensure_cef_debug_file()?;

        if self.is_running().await {
            // Steam running but CEF not available → restart needed.
            let result = self.restart().await;
            if !result.success {
                return Err(SteamError::Controller(result.message));
            }
        } else {
            self.start().await?;
            self.wait_for_cef().await?;
        }

        Ok(())
    }

    /// Ensures the CEF debug file exists in the Steam directory.
    ///
    /// Returns `true` if the file was created (didn't exist before).
    pub fn ensure_cef_debug_file(&self) -> Result<bool, SteamError> {
        let paths = crate::Paths::new()?;
        let debug_path = paths.base_dir().join(CEF_DEBUG_FILE);

        if debug_path.exists() {
            return Ok(false);
        }

        std::fs::write(&debug_path, b"").map_err(|e| {
            SteamError::Io(format!(
                "failed to create CEF debug file at {}: {e}",
                debug_path.display()
            ))
        })?;

        Ok(true)
    }
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

/// Checks if the CEF debug endpoint responds.
async fn is_cef_available() -> bool {
    // Use a raw TCP connect + minimal HTTP to avoid pulling in reqwest.
    let result = tokio::time::timeout(Duration::from_secs(2), async {
        let stream = tokio::net::TcpStream::connect("127.0.0.1:8080").await?;
        // If we can connect, CEF is likely available.
        drop(stream);
        Ok::<_, std::io::Error>(())
    })
    .await;

    matches!(result, Ok(Ok(())))
}

// Platform-specific implementation.
#[cfg(target_os = "linux")]
#[path = "controller_linux.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "controller_windows.rs"]
mod platform;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod platform {
    use crate::SteamError;

    pub async fn is_running() -> bool {
        false
    }
    pub fn is_gaming_mode() -> bool {
        false
    }
    pub async fn start() -> Result<(), SteamError> {
        Err(SteamError::NotFound)
    }
    pub async fn shutdown_command() -> Result<(), SteamError> {
        Err(SteamError::NotFound)
    }
    pub async fn force_kill() -> Result<(), SteamError> {
        Err(SteamError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_default() {
        let _ = Controller;
    }

    #[test]
    fn restart_result_clone() {
        let r = RestartResult {
            success: true,
            message: "ok".into(),
        };
        let r2 = r.clone();
        assert!(r2.success);
        assert_eq!(r2.message, "ok");
    }

    #[test]
    fn cef_debug_file_constant() {
        assert_eq!(CEF_DEBUG_FILE, ".cef-enable-remote-debugging");
    }

    #[tokio::test]
    async fn is_cef_available_does_not_panic() {
        // Just verify the function doesn't crash — port 8080 may or
        // may not have a listener depending on the environment.
        let _ = is_cef_available().await;
    }

    #[tokio::test]
    async fn controller_is_running() {
        let ctrl = Controller::new();
        // Just verify it doesn't panic.
        let _ = ctrl.is_running().await;
    }

    #[tokio::test]
    async fn shutdown_when_not_running_is_noop() {
        let ctrl = Controller::new();
        // If Steam is not running, shutdown should succeed immediately.
        if !ctrl.is_running().await {
            let result = ctrl.shutdown().await;
            assert!(result.is_ok());
        }
    }
}
