//! Wrapper script injection/removal in Steam launch options.
//!
//! Manages the lifecycle of wrapper scripts for individual games,
//! tracks which games have wrappers enabled, and manipulates
//! Steam launch options via CEF.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use capydeploy_protocol::telemetry::{GameLogWrapperStatusEvent, SetGameLogWrapperResponse};
use capydeploy_steam::CefClient;
use tokio::sync::Mutex;

use crate::WRAPPER_SCRIPT;

/// Manages game log wrapper scripts and their Steam launch option injection.
pub struct WrapperManager {
    inner: Arc<Mutex<WrapperState>>,
}

struct WrapperState {
    /// Map of appID → enabled state.
    wrappers: HashMap<u32, bool>,
    /// Path to the installed wrapper script.
    script_path: PathBuf,
}

impl WrapperManager {
    /// Creates a new wrapper manager.
    ///
    /// The wrapper script is installed to `base_dir/capydeploy-game-wrapper.sh`.
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(WrapperState {
                wrappers: HashMap::new(),
                script_path: base_dir.join("capydeploy-game-wrapper.sh"),
            })),
        }
    }

    /// Ensures the wrapper script is installed and executable.
    pub async fn ensure_script_installed(&self) -> Result<PathBuf, String> {
        let state = self.inner.lock().await;
        let path = &state.script_path;

        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create script directory: {e}"))?;
            }
            let mut file = std::fs::File::create(path)
                .map_err(|e| format!("failed to create wrapper script: {e}"))?;
            file.write_all(WRAPPER_SCRIPT.as_bytes())
                .map_err(|e| format!("failed to write wrapper script: {e}"))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
                    .map_err(|e| format!("failed to set script permissions: {e}"))?;
            }

            tracing::info!(path = %path.display(), "wrapper script installed");
        }

        Ok(path.clone())
    }

    /// Enables or disables the wrapper for a game.
    ///
    /// Returns the response confirming the new state.
    pub async fn set_wrapper(
        &self,
        app_id: u32,
        enabled: bool,
    ) -> Result<SetGameLogWrapperResponse, String> {
        let script_path = self.ensure_script_installed().await?;

        if enabled {
            inject_wrapper(app_id, &script_path).await?;
        } else {
            strip_wrapper(app_id, &script_path).await?;
        }

        let mut state = self.inner.lock().await;
        if enabled {
            state.wrappers.insert(app_id, true);
        } else {
            state.wrappers.remove(&app_id);
        }

        tracing::info!(app_id, enabled, "game log wrapper updated");

        Ok(SetGameLogWrapperResponse { app_id, enabled })
    }

    /// Returns the current wrapper status for all games.
    pub async fn status(&self) -> GameLogWrapperStatusEvent {
        let state = self.inner.lock().await;
        GameLogWrapperStatusEvent {
            wrappers: state.wrappers.clone(),
        }
    }

    /// Returns whether the wrapper is enabled for a specific game.
    pub async fn is_enabled(&self, app_id: u32) -> bool {
        let state = self.inner.lock().await;
        state.wrappers.get(&app_id).copied().unwrap_or(false)
    }
}

/// Builds the wrapper prefix for launch options.
fn wrapper_prefix(script_path: &std::path::Path, app_id: u32) -> String {
    format!("{} {} ", script_path.display(), app_id)
}

/// Injects the wrapper script into Steam launch options via CEF.
async fn inject_wrapper(app_id: u32, script_path: &std::path::Path) -> Result<(), String> {
    let client = CefClient::new();
    let prefix = wrapper_prefix(script_path, app_id);

    // Read current launch options.
    let current = read_launch_options(&client, app_id).await?;

    // Check if wrapper is already injected.
    if current.contains(&prefix) {
        tracing::debug!(app_id, "wrapper already present in launch options");
        return Ok(());
    }

    // Prepend wrapper to existing options, preserving %command%.
    let new_options = if current.is_empty() {
        format!("{prefix}%command%")
    } else {
        format!("{prefix}{current}")
    };

    set_launch_options(&client, app_id, &new_options).await?;
    tracing::debug!(app_id, options = %new_options, "wrapper injected");

    Ok(())
}

/// Strips the wrapper script from Steam launch options via CEF.
async fn strip_wrapper(app_id: u32, script_path: &std::path::Path) -> Result<(), String> {
    let client = CefClient::new();
    let prefix = wrapper_prefix(script_path, app_id);

    let current = read_launch_options(&client, app_id).await?;

    if !current.contains(&prefix) {
        tracing::debug!(app_id, "wrapper not present in launch options");
        return Ok(());
    }

    let new_options = current.replace(&prefix, "");
    set_launch_options(&client, app_id, &new_options).await?;
    tracing::debug!(app_id, options = %new_options, "wrapper stripped");

    Ok(())
}

/// Reads current launch options for an app via CEF.
async fn read_launch_options(client: &CefClient, app_id: u32) -> Result<String, String> {
    let js = format!(
        r#"(async () => {{
            const details = await SteamClient.Apps.GetLaunchOptionsForApp({app_id});
            return details || "";
        }})()"#
    );

    let result = client
        .evaluate(&js)
        .await
        .map_err(|e| format!("failed to read launch options for {app_id}: {e}"))?;

    Ok(result.as_str().unwrap_or("").to_string())
}

/// Sets launch options for an app via CEF.
async fn set_launch_options(client: &CefClient, app_id: u32, options: &str) -> Result<(), String> {
    let escaped = serde_json::to_string(options).unwrap_or_else(|_| "\"\"".to_string());
    let js = format!("SteamClient.Apps.SetShortcutLaunchOptions({app_id}, {escaped})");

    client
        .evaluate_void(&js)
        .await
        .map_err(|e| format!("failed to set launch options for {app_id}: {e}"))?;

    Ok(())
}

/// Returns the log directory path.
pub fn log_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home)
        .join(".local/share")
        .join(crate::LOG_DIR_NAME)
}

/// Returns the expected log file pattern for a given appID.
pub fn log_file_pattern(app_id: u32) -> String {
    format!("game_{app_id}_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_prefix_format() {
        let path = std::path::Path::new("/opt/capydeploy/capydeploy-game-wrapper.sh");
        let prefix = wrapper_prefix(path, 12345);
        assert_eq!(prefix, "/opt/capydeploy/capydeploy-game-wrapper.sh 12345 ");
    }

    #[test]
    fn log_dir_uses_home() {
        // SAFETY: Test-only, single-threaded context.
        unsafe { std::env::set_var("HOME", "/test/home") };
        let dir = log_dir();
        assert_eq!(
            dir,
            PathBuf::from("/test/home/.local/share/capydeploy/logs")
        );
    }

    #[test]
    fn log_file_pattern_format() {
        assert_eq!(log_file_pattern(12345), "game_12345_");
        assert_eq!(log_file_pattern(0), "game_0_");
    }

    #[tokio::test]
    async fn wrapper_manager_status_empty() {
        let manager = WrapperManager::new(PathBuf::from("/tmp/test-capydeploy"));
        let status = manager.status().await;
        assert!(status.wrappers.is_empty());
    }

    #[tokio::test]
    async fn wrapper_manager_is_enabled_default() {
        let manager = WrapperManager::new(PathBuf::from("/tmp/test-capydeploy"));
        assert!(!manager.is_enabled(12345).await);
    }

    #[tokio::test]
    async fn ensure_script_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = WrapperManager::new(tmp.path().to_path_buf());
        let path = manager.ensure_script_installed().await.unwrap();

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("exec \"$@\""));
        assert!(content.contains("capydeploy/logs"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o755, 0o755);
        }
    }

    #[tokio::test]
    async fn ensure_script_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = WrapperManager::new(tmp.path().to_path_buf());

        let path1 = manager.ensure_script_installed().await.unwrap();
        let path2 = manager.ensure_script_installed().await.unwrap();
        assert_eq!(path1, path2);
    }
}
