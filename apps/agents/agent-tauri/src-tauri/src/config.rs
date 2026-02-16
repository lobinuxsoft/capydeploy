//! Agent configuration management.
//!
//! Reads/writes JSON at `~/.config/capydeploy-agent/config.json`,
//! matching the Go agent's format for backward compatibility.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Authorized Hub stored in config (matches Go `config.AuthorizedHub`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedHub {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub platform: String,
    pub token: String,
    pub paired_at: String,
    pub last_seen: String,
}

/// On-disk config format (matches Go `config.Config`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigFile {
    #[serde(default)]
    name: String,
    #[serde(default)]
    install_path: String,
    #[serde(default)]
    telemetry_enabled: bool,
    #[serde(default)]
    telemetry_interval: i32,
    #[serde(default)]
    console_log_enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    authorized_hubs: Vec<AuthorizedHub>,
}

/// Agent configuration.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub install_path: String,
    pub telemetry_enabled: bool,
    pub telemetry_interval: i32,
    pub console_log_enabled: bool,
    pub authorized_hubs: Vec<AuthorizedHub>,
    file_path: PathBuf,
}

fn default_name() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "CapyDeploy Agent".into())
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: default_name(),
            install_path: "~/Games".into(),
            telemetry_enabled: false,
            telemetry_interval: 2,
            console_log_enabled: false,
            authorized_hubs: Vec::new(),
            file_path: config_file_path().unwrap_or_else(|_| PathBuf::from("/tmp/config.json")),
        }
    }
}

impl AgentConfig {
    /// Loads configuration from disk.
    pub fn load() -> anyhow::Result<Self> {
        let file_path = config_file_path()?;
        let mut config = AgentConfig {
            file_path: file_path.clone(),
            ..Default::default()
        };

        if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)?;
            if let Ok(file) = serde_json::from_str::<ConfigFile>(&content) {
                if !file.name.is_empty() {
                    config.name = file.name;
                }
                if !file.install_path.is_empty() {
                    config.install_path = file.install_path;
                }
                config.telemetry_enabled = file.telemetry_enabled;
                if file.telemetry_interval >= 1 && file.telemetry_interval <= 10 {
                    config.telemetry_interval = file.telemetry_interval;
                }
                config.console_log_enabled = file.console_log_enabled;
                config.authorized_hubs = file.authorized_hubs;
            } else {
                tracing::warn!(
                    path = %file_path.display(),
                    "failed to parse agent config, using defaults"
                );
            }
        }

        Ok(config)
    }

    /// Saves configuration to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = ConfigFile {
            name: self.name.clone(),
            install_path: self.install_path.clone(),
            telemetry_enabled: self.telemetry_enabled,
            telemetry_interval: self.telemetry_interval,
            console_log_enabled: self.console_log_enabled,
            authorized_hubs: self.authorized_hubs.clone(),
        };

        let json = serde_json::to_string_pretty(&file)?;
        std::fs::write(&self.file_path, &json)?;
        set_permissions_0600(&self.file_path);

        tracing::debug!("agent configuration saved");
        Ok(())
    }

    /// Adds or updates an authorized Hub.
    pub fn add_authorized_hub(&mut self, hub: AuthorizedHub) {
        if let Some(existing) = self.authorized_hubs.iter_mut().find(|h| h.id == hub.id) {
            *existing = hub;
        } else {
            self.authorized_hubs.push(hub);
        }
    }

    /// Removes an authorized Hub by ID.
    pub fn remove_authorized_hub(&mut self, hub_id: &str) {
        self.authorized_hubs.retain(|h| h.id != hub_id);
    }

    /// Updates the last_seen timestamp for a Hub.
    pub fn update_hub_last_seen(&mut self, hub_id: &str, last_seen: &str) {
        if let Some(hub) = self.authorized_hubs.iter_mut().find(|h| h.id == hub_id) {
            hub.last_seen = last_seen.to_string();
        }
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

fn config_file_path() -> anyhow::Result<PathBuf> {
    let config_dir = config_base_dir()?;
    Ok(config_dir.join("capydeploy-agent").join("config.json"))
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
