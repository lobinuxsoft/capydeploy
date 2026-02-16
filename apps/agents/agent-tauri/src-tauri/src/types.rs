//! DTOs for frontend communication.
//!
//! These structs use `#[serde(rename_all = "camelCase")]` to match
//! the JSON shapes that the Svelte frontend expects (originally from Go).

use serde::{Deserialize, Serialize};

/// Agent status sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusDto {
    pub running: bool,
    pub name: String,
    pub platform: String,
    pub version: String,
    pub port: u16,
    pub ips: Vec<String>,
    pub accept_connections: bool,
    pub connected_hub: Option<ConnectedHubDto>,
    pub telemetry_enabled: bool,
    pub telemetry_interval: i32,
    pub console_log_enabled: bool,
}

/// Connected Hub info for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedHubDto {
    pub id: String,
    pub name: String,
    pub ip: String,
}

/// Authorized Hub info for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedHubDto {
    pub id: String,
    pub name: String,
    pub paired_at: String,
    pub last_seen: String,
}

/// Version info DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfoDto {
    pub version: String,
    pub commit: String,
    pub build_date: String,
}

/// Steam user info for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamUserDto {
    pub id: String,
    pub name: String,
}

/// Shortcut info for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutDto {
    pub app_id: u32,
    pub name: String,
    pub exe: String,
    pub start_dir: String,
}

/// Operation event for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEventDto {
    #[serde(rename = "type")]
    pub event_type: String,
    pub status: String,
    pub game_name: String,
    pub progress: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
}
