//! DTOs for frontend communication.
//!
//! These structs use `#[serde(rename_all = "camelCase")]` to match
//! the JSON shapes that the Svelte frontend expects (originally from Go).

use serde::{Deserialize, Serialize};

/// Discovered agent info sent to the frontend.
///
/// `DiscoveredAgent` has `#[serde(skip)]` on `ips`, `discovered_at`, `last_seen`,
/// so we must build this DTO manually.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredAgentDto {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub version: String,
    pub host: String,
    pub port: u16,
    pub ips: Vec<String>,
    pub discovered_at: String,
    pub last_seen: String,
    pub online: bool,
}

impl From<&capydeploy_discovery::types::DiscoveredAgent> for DiscoveredAgentDto {
    fn from(a: &capydeploy_discovery::types::DiscoveredAgent) -> Self {
        Self {
            id: a.info.id.clone(),
            name: a.info.name.clone(),
            platform: a.info.platform.clone(),
            version: a.info.version.clone(),
            host: a.host.clone(),
            port: a.port,
            ips: a.ips.iter().map(|ip| ip.to_string()).collect(),
            discovered_at: a
                .discovered_at
                .map(|t| format!("{:.0}s ago", t.elapsed().as_secs_f64()))
                .unwrap_or_default(),
            last_seen: a
                .last_seen
                .map(|t| format!("{:.0}s ago", t.elapsed().as_secs_f64()))
                .unwrap_or_default(),
            online: a.last_seen.is_some_and(|t| t.elapsed().as_secs() < 120),
        }
    }
}

/// Connection status sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatusDto {
    pub connected: bool,
    pub agent_id: String,
    pub agent_name: String,
    pub platform: String,
    pub host: String,
    pub port: u16,
    pub ips: Vec<String>,
    pub supported_image_formats: Vec<String>,
}

impl ConnectionStatusDto {
    pub fn disconnected() -> Self {
        Self {
            connected: false,
            agent_id: String::new(),
            agent_name: String::new(),
            platform: String::new(),
            host: String::new(),
            port: 0,
            ips: Vec::new(),
            supported_image_formats: Vec::new(),
        }
    }

    pub fn from_connected(agent: &capydeploy_hub_connection::ConnectedAgent) -> Self {
        Self {
            connected: true,
            agent_id: agent.agent.info.id.clone(),
            agent_name: agent.agent.info.name.clone(),
            platform: agent.agent.info.platform.clone(),
            host: agent.agent.host.clone(),
            port: agent.agent.port,
            ips: agent.agent.ips.iter().map(|ip| ip.to_string()).collect(),
            supported_image_formats: agent.agent.info.supported_image_formats.clone(),
        }
    }
}

/// Version info DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfoDto {
    pub version: String,
    pub commit: String,
    pub build_date: String,
}

/// Hub identity DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubInfoDto {
    pub id: String,
    pub name: String,
    pub platform: String,
}

/// Pairing event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingRequiredDto {
    pub agent_id: String,
    pub code: String,
    pub expires_in: i32,
}

/// Upload progress DTO matching frontend expectations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgressDto {
    pub progress: f64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub done: bool,
}

/// Installed game DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledGameDto {
    pub name: String,
    pub path: String,
    pub size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<u32>,
}

/// Artwork file result from local file selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkFileResultDto {
    pub path: String,
    pub data_uri: String,
    pub content_type: String,
    pub size: u64,
}

/// Reconnecting event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconnectingDto {
    pub agent_id: String,
    pub attempt: u32,
    pub next_retry_secs: f64,
}

/// SteamGridDB image filters (received from frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageFiltersDto {
    pub style: String,
    pub mime_type: String,
    pub image_type: String,
    pub dimension: String,
    pub show_nsfw: bool,
    pub show_humor: bool,
}

impl From<ImageFiltersDto> for capydeploy_steamgriddb::ImageFilters {
    fn from(f: ImageFiltersDto) -> Self {
        Self {
            style: f.style,
            mime_type: f.mime_type,
            image_type: f.image_type,
            dimension: f.dimension,
            show_nsfw: f.show_nsfw,
            show_humor: f.show_humor,
        }
    }
}
