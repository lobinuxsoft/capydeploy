//! Agent configuration management.
//!
//! Configuration is stored as TOML:
//! - Linux: `~/.config/capydeploy/agent.toml`
//! - Windows: `%APPDATA%/capydeploy/agent.toml`

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Display name of this agent (hostname by default).
    #[serde(default = "default_name")]
    pub name: String,

    /// Base path for game installations.
    #[serde(default = "default_install_path")]
    pub install_path: String,

    /// WebSocket server port (0 = auto-assign).
    #[serde(default = "default_port")]
    pub port: u16,

    /// Enable telemetry collection on start.
    #[serde(default = "default_true")]
    pub telemetry_enabled: bool,

    /// Telemetry collection interval in seconds.
    #[serde(default = "default_telemetry_interval")]
    pub telemetry_interval: u32,

    /// Enable console log streaming on start.
    #[serde(default)]
    pub console_log_enabled: bool,
}

fn default_name() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "CapyDeploy Agent".into())
}

fn default_install_path() -> String {
    "~/Games".into()
}

fn default_port() -> u16 {
    0
}

fn default_true() -> bool {
    true
}

fn default_telemetry_interval() -> u32 {
    2
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: default_name(),
            install_path: default_install_path(),
            port: default_port(),
            telemetry_enabled: default_true(),
            telemetry_interval: default_telemetry_interval(),
            console_log_enabled: false,
        }
    }
}

impl Config {
    /// Loads configuration from disk, or creates a default if not found.
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
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

        // Restrict permissions on Unix.
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
            .join("agent.toml"))
    }

    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData".into());
        Ok(PathBuf::from(appdata).join("capydeploy").join("agent.toml"))
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok(PathBuf::from("/tmp/capydeploy/agent.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = Config::default();
        assert!(!config.name.is_empty());
        assert_eq!(config.install_path, "~/Games");
        assert_eq!(config.port, 0);
        assert!(config.telemetry_enabled);
        assert_eq!(config.telemetry_interval, 2);
        assert!(!config.console_log_enabled);
    }

    #[test]
    fn config_roundtrip_toml() {
        let config = Config {
            name: "TestAgent".into(),
            install_path: "/opt/games".into(),
            port: 8888,
            telemetry_enabled: false,
            telemetry_interval: 5,
            console_log_enabled: true,
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.name, "TestAgent");
        assert_eq!(parsed.install_path, "/opt/games");
        assert_eq!(parsed.port, 8888);
        assert!(!parsed.telemetry_enabled);
        assert_eq!(parsed.telemetry_interval, 5);
        assert!(parsed.console_log_enabled);
    }

    #[test]
    fn config_partial_toml() {
        // Only specify name, rest should use defaults.
        let toml_str = r#"name = "MyAgent""#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "MyAgent");
        assert_eq!(config.install_path, "~/Games");
        assert!(config.telemetry_enabled);
    }

    #[test]
    fn config_path_not_empty() {
        let path = config_path().unwrap();
        assert!(path.to_string_lossy().contains("capydeploy"));
    }

    #[test]
    fn config_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("agent.toml");

        let config = Config {
            name: "SaveTest".into(),
            ..Config::default()
        };

        // Write manually since save() uses config_path().
        let content = toml::to_string_pretty(&config).unwrap();
        std::fs::write(&path, &content).unwrap();

        // Read back.
        let loaded_content = std::fs::read_to_string(&path).unwrap();
        let loaded: Config = toml::from_str(&loaded_content).unwrap();
        assert_eq!(loaded.name, "SaveTest");
    }
}
