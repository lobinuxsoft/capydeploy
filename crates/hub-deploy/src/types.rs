//! Data types for the deploy flow.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A saved game installation setup (Hub-side config).
///
/// Mirrors Go's `config.GameSetup`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSetup {
    pub id: String,
    pub name: String,
    pub local_path: String,
    pub executable: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub launch_options: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tags: String,
    pub install_path: String,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub griddb_game_id: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub grid_portrait: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub grid_landscape: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hero_image: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub logo_image: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon_image: String,
}

fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}

/// Classification of an artwork source path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ArtworkSource {
    /// Remote HTTP(S) URL — sent as part of ShortcutConfig.
    Remote(String),
    /// Local file:// path — sent as binary WS message.
    Local(String),
    /// No artwork configured.
    #[default]
    None,
}

/// Artwork assignment for a game setup — one entry per artwork slot.
#[derive(Debug, Clone, Default)]
pub struct ArtworkAssignment {
    pub grid: ArtworkSource,
    pub banner: ArtworkSource,
    pub hero: ArtworkSource,
    pub logo: ArtworkSource,
    pub icon: ArtworkSource,
}

/// A local artwork file to send via binary WS message.
#[derive(Debug, Clone)]
pub struct LocalArtwork {
    pub art_type: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

/// Progress event emitted during deployment.
#[derive(Debug, Clone)]
pub enum DeployEvent {
    /// Progress update.
    Progress {
        agent_id: String,
        progress: f64,
        status: String,
    },
    /// Deployment completed successfully for an agent.
    Completed { agent_id: String },
    /// Deployment failed for an agent.
    Failed { agent_id: String, error: String },
}

/// Result of a single agent deployment.
#[derive(Debug, Clone)]
pub struct DeployResult {
    pub agent_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub app_id: Option<u32>,
}

/// Config passed to the deploy orchestrator.
#[derive(Debug, Clone)]
pub struct DeployConfig {
    pub setup: GameSetup,
    pub artwork: ArtworkAssignment,
}

/// Response from InitUpload on the agent side.
#[derive(Debug, Clone)]
pub struct InitUploadResult {
    pub upload_id: String,
    pub chunk_size: i32,
    pub resume_from: Option<HashMap<String, i64>>,
    /// TCP data channel port advertised by the agent (None = no TCP support).
    pub tcp_port: Option<u16>,
    /// TCP data channel authentication token.
    pub tcp_token: Option<String>,
}

/// Response from CompleteUpload on the agent side.
#[derive(Debug, Clone)]
pub struct CompleteUploadResult {
    pub success: bool,
    pub path: String,
    pub app_id: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_setup_json_roundtrip() {
        let setup = GameSetup {
            id: "game_1".into(),
            name: "Test Game".into(),
            local_path: "/games/test".into(),
            executable: "game.exe".into(),
            launch_options: String::new(),
            tags: "Action, RPG".into(),
            install_path: "/home/deck/Games/test".into(),
            griddb_game_id: 0,
            grid_portrait: String::new(),
            grid_landscape: String::new(),
            hero_image: String::new(),
            logo_image: String::new(),
            icon_image: String::new(),
        };
        let json = serde_json::to_string(&setup).unwrap();
        assert!(!json.contains("launch_options"));
        assert!(!json.contains("griddb_game_id"));
        let parsed: GameSetup = serde_json::from_str(&json).unwrap();
        assert_eq!(setup, parsed);
    }
}
