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
    pub fn load() -> anyhow::Result<Self> {
        let mut config = HubConfig::default();

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

/// Returns the token store path matching the Go Hub: `capydeploy-hub/tokens.json`.
pub fn token_store_path() -> Option<PathBuf> {
    config_base_dir()
        .ok()
        .map(|d| d.join("capydeploy-hub").join("tokens.json"))
}

fn hub_identity_path() -> anyhow::Result<PathBuf> {
    let config_dir = config_base_dir()?;
    Ok(config_dir.join("capydeploy-hub").join("config.json"))
}

fn app_config_path() -> anyhow::Result<PathBuf> {
    let config_dir = config_base_dir()?;
    Ok(config_dir.join("capydeploy").join("config.json"))
}

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
