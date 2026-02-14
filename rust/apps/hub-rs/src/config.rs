//! Hub configuration management.
//!
//! Configuration is stored as TOML:
//! - Linux: `~/.config/capydeploy/hub.toml`
//! - Windows: `%APPDATA%/capydeploy/hub.toml`

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Hub configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubConfig {
    /// Display name of this Hub (hostname by default).
    #[serde(default = "default_name")]
    pub name: String,

    /// SteamGridDB API key for artwork search.
    #[serde(default)]
    pub steamgriddb_api_key: String,

    /// Directory for game log files.
    #[serde(default)]
    pub game_log_dir: String,

    /// Saved game installation setups.
    #[serde(default)]
    pub game_setups: Vec<capydeploy_hub_deploy::GameSetup>,
}

fn default_name() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "CapyDeploy Hub".into())
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            name: default_name(),
            steamgriddb_api_key: String::new(),
            game_log_dir: String::new(),
            game_setups: Vec::new(),
        }
    }
}

impl HubConfig {
    /// Loads configuration from disk, or creates a default if not found.
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: HubConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = HubConfig::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Saves the current configuration to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;

        // Restrict permissions on Unix (may contain API key).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }

        tracing::debug!(path = %path.display(), "configuration saved");
        Ok(())
    }
}

/// Returns the platform-specific configuration file path.
fn config_path() -> anyhow::Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        Ok(PathBuf::from(home)
            .join(".config")
            .join("capydeploy")
            .join("hub.toml"))
    }

    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData".into());
        Ok(PathBuf::from(appdata).join("capydeploy").join("hub.toml"))
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok(PathBuf::from("/tmp/capydeploy/hub.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = HubConfig::default();
        assert!(!config.name.is_empty());
        assert!(config.steamgriddb_api_key.is_empty());
        assert!(config.game_setups.is_empty());
    }

    #[test]
    fn config_roundtrip_toml() {
        let config = HubConfig {
            name: "Test Hub".into(),
            steamgriddb_api_key: "abc123".into(),
            game_log_dir: "/tmp/logs".into(),
            game_setups: Vec::new(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: HubConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.name, "Test Hub");
        assert_eq!(parsed.steamgriddb_api_key, "abc123");
        assert_eq!(parsed.game_log_dir, "/tmp/logs");
    }

    #[test]
    fn config_partial_toml() {
        let toml_str = r#"name = "MyHub""#;
        let config: HubConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "MyHub");
        assert!(config.steamgriddb_api_key.is_empty());
    }

    #[test]
    fn config_path_not_empty() {
        let path = config_path().unwrap();
        assert!(path.to_string_lossy().contains("capydeploy"));
    }

    #[test]
    fn config_save_and_load_manual() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hub.toml");

        let config = HubConfig {
            name: "SaveTest".into(),
            ..HubConfig::default()
        };

        let content = toml::to_string_pretty(&config).unwrap();
        std::fs::write(&path, &content).unwrap();

        let loaded_content = std::fs::read_to_string(&path).unwrap();
        let loaded: HubConfig = toml::from_str(&loaded_content).unwrap();
        assert_eq!(loaded.name, "SaveTest");
    }
}
