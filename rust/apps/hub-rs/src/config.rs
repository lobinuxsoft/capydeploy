//! Hub configuration management.
//!
//! Reads the same JSON files as the Go Hub for backward compatibility:
//! - Hub identity: `~/.config/capydeploy-hub/config.json`
//! - App config:   `~/.config/capydeploy/config.json`

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Hub identity (capydeploy-hub/config.json)
// ---------------------------------------------------------------------------

/// Hub identity — mirrors Go's `apps/hub/config.Config`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HubIdentityFile {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    platform: String,
}

// ---------------------------------------------------------------------------
// App config (capydeploy/config.json)
// ---------------------------------------------------------------------------

/// App config — mirrors Go's `pkg/config.AppConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfigFile {
    #[serde(default)]
    game_setups: Vec<capydeploy_hub_deploy::GameSetup>,
    #[serde(default)]
    steamgriddb_api_key: String,
    #[serde(default)]
    image_cache_enabled: bool,
    #[serde(default)]
    game_log_directory: String,
}

// ---------------------------------------------------------------------------
// Unified HubConfig
// ---------------------------------------------------------------------------

/// Hub configuration — unified view of both JSON files.
#[derive(Debug, Clone)]
pub struct HubConfig {
    /// Display name of this Hub.
    pub name: String,

    /// Stable Hub identifier (from capydeploy-hub/config.json).
    pub hub_id: String,

    /// SteamGridDB API key for artwork search.
    pub steamgriddb_api_key: String,

    /// Directory for game log files.
    pub game_log_dir: String,

    /// Saved game installation setups.
    pub game_setups: Vec<capydeploy_hub_deploy::GameSetup>,
}

fn default_name() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "CapyDeploy Hub".into())
}

fn default_hub_id() -> String {
    // Match Go's generateID: sha256(hostname-platform-hub)[:8]
    use std::fmt::Write;
    let hostname = default_name();
    let platform = std::env::consts::OS;
    let data = format!("{hostname}-{platform}-hub");
    let digest = <sha2::Sha256 as sha2::Digest>::digest(data.as_bytes());
    let mut hex = String::with_capacity(8);
    for byte in &digest[..4] {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            name: default_name(),
            hub_id: default_hub_id(),
            steamgriddb_api_key: String::new(),
            game_log_dir: String::new(),
            game_setups: Vec::new(),
        }
    }
}

impl HubConfig {
    /// Loads configuration from the Go Hub's JSON files.
    ///
    /// Reads both `capydeploy-hub/config.json` (identity) and
    /// `capydeploy/config.json` (app config). Missing files are
    /// silently ignored and defaults are used.
    pub fn load() -> anyhow::Result<Self> {
        let mut config = HubConfig::default();

        // 1. Hub identity file.
        let identity_path = hub_identity_path()?;
        if identity_path.exists() {
            let content = std::fs::read_to_string(&identity_path)?;
            if let Ok(identity) = serde_json::from_str::<HubIdentityFile>(&content) {
                if !identity.id.is_empty() {
                    config.hub_id = identity.id;
                }
                if !identity.name.is_empty() {
                    config.name = identity.name;
                }
            } else {
                tracing::warn!(
                    path = %identity_path.display(),
                    "failed to parse hub identity, using defaults"
                );
            }
        }

        // 2. App config file.
        let app_path = app_config_path()?;
        if app_path.exists() {
            let content = std::fs::read_to_string(&app_path)?;
            if let Ok(app) = serde_json::from_str::<AppConfigFile>(&content) {
                config.steamgriddb_api_key = app.steamgriddb_api_key;
                config.game_log_dir = app.game_log_directory;
                config.game_setups = app.game_setups;
            } else {
                tracing::warn!(
                    path = %app_path.display(),
                    "failed to parse app config, using defaults"
                );
            }
        }

        Ok(config)
    }

    /// Saves configuration back to both JSON files.
    pub fn save(&self) -> anyhow::Result<()> {
        // Save hub identity.
        let identity_path = hub_identity_path()?;
        if let Some(parent) = identity_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let identity = HubIdentityFile {
            id: self.hub_id.clone(),
            name: self.name.clone(),
            platform: std::env::consts::OS.into(),
        };
        let identity_json = serde_json::to_string_pretty(&identity)?;
        std::fs::write(&identity_path, &identity_json)?;
        set_permissions_0600(&identity_path);

        // Save app config.
        let app_path = app_config_path()?;
        if let Some(parent) = app_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let app = AppConfigFile {
            game_setups: self.game_setups.clone(),
            steamgriddb_api_key: self.steamgriddb_api_key.clone(),
            image_cache_enabled: true,
            game_log_directory: self.game_log_dir.clone(),
        };
        let app_json = serde_json::to_string_pretty(&app)?;
        std::fs::write(&app_path, &app_json)?;
        set_permissions_0600(&app_path);

        tracing::debug!("configuration saved");
        Ok(())
    }
}

/// Restricts file permissions to owner-only on Unix.
fn set_permissions_0600(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// Hub identity file: `~/.config/capydeploy-hub/config.json`
fn hub_identity_path() -> anyhow::Result<PathBuf> {
    let config_dir = config_base_dir()?;
    Ok(config_dir.join("capydeploy-hub").join("config.json"))
}

/// App config file: `~/.config/capydeploy/config.json`
fn app_config_path() -> anyhow::Result<PathBuf> {
    let config_dir = config_base_dir()?;
    Ok(config_dir.join("capydeploy").join("config.json"))
}

/// Platform-specific config base directory.
fn config_base_dir() -> anyhow::Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        Ok(PathBuf::from(home).join(".config"))
    }

    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData".into());
        Ok(PathBuf::from(appdata))
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok(PathBuf::from("/tmp"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = HubConfig::default();
        assert!(!config.name.is_empty());
        assert!(!config.hub_id.is_empty());
        assert_eq!(config.hub_id.len(), 8); // SHA256[:8] hex
        assert!(config.steamgriddb_api_key.is_empty());
        assert!(config.game_setups.is_empty());
    }

    #[test]
    fn parse_hub_identity_json() {
        let json = r#"{"id":"a1b2c3d4","name":"MyHub","platform":"linux"}"#;
        let identity: HubIdentityFile = serde_json::from_str(json).unwrap();
        assert_eq!(identity.id, "a1b2c3d4");
        assert_eq!(identity.name, "MyHub");
        assert_eq!(identity.platform, "linux");
    }

    #[test]
    fn parse_app_config_json() {
        let json = r#"{
            "game_setups": [
                {
                    "id": "game_1",
                    "name": "Portal 2",
                    "local_path": "/games/portal2",
                    "executable": "portal2.exe",
                    "install_path": ""
                }
            ],
            "steamgriddb_api_key": "abc123",
            "image_cache_enabled": true,
            "game_log_directory": "/tmp/logs"
        }"#;
        let app: AppConfigFile = serde_json::from_str(json).unwrap();
        assert_eq!(app.game_setups.len(), 1);
        assert_eq!(app.game_setups[0].name, "Portal 2");
        assert_eq!(app.steamgriddb_api_key, "abc123");
        assert_eq!(app.game_log_directory, "/tmp/logs");
        assert!(app.image_cache_enabled);
    }

    #[test]
    fn parse_app_config_empty() {
        let json = "{}";
        let app: AppConfigFile = serde_json::from_str(json).unwrap();
        assert!(app.game_setups.is_empty());
        assert!(app.steamgriddb_api_key.is_empty());
    }

    #[test]
    fn hub_id_is_stable_sha256() {
        let id1 = default_hub_id();
        let id2 = default_hub_id();
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 8);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let identity_dir = tmp.path().join("capydeploy-hub");
        let app_dir = tmp.path().join("capydeploy");
        std::fs::create_dir_all(&identity_dir).unwrap();
        std::fs::create_dir_all(&app_dir).unwrap();

        // Write identity.
        let identity = HubIdentityFile {
            id: "test-id".into(),
            name: "Test Hub".into(),
            platform: "linux".into(),
        };
        std::fs::write(
            identity_dir.join("config.json"),
            serde_json::to_string_pretty(&identity).unwrap(),
        )
        .unwrap();

        // Write app config.
        let app = AppConfigFile {
            game_setups: Vec::new(),
            steamgriddb_api_key: "key123".into(),
            image_cache_enabled: true,
            game_log_directory: "/logs".into(),
        };
        std::fs::write(
            app_dir.join("config.json"),
            serde_json::to_string_pretty(&app).unwrap(),
        )
        .unwrap();

        // Parse them manually (can't use load() since it uses real paths).
        let id_content = std::fs::read_to_string(identity_dir.join("config.json")).unwrap();
        let parsed_id: HubIdentityFile = serde_json::from_str(&id_content).unwrap();
        assert_eq!(parsed_id.id, "test-id");

        let app_content = std::fs::read_to_string(app_dir.join("config.json")).unwrap();
        let parsed_app: AppConfigFile = serde_json::from_str(&app_content).unwrap();
        assert_eq!(parsed_app.steamgriddb_api_key, "key123");
    }
}
