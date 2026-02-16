use serde::{Deserialize, Serialize};

/// Information about a discovered agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub version: String,
    pub accept_connections: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_image_formats: Vec<String>,
}

/// Configuration for uploading a game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadConfig {
    pub game_name: String,
    pub install_path: String,
    pub executable: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub launch_options: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tags: String,
}

/// Configuration for creating a Steam shortcut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutConfig {
    pub name: String,
    pub exe: String,
    pub start_dir: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub launch_options: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artwork: Option<ArtworkConfig>,
}

/// Artwork paths for a shortcut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtworkConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub grid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hero: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub logo: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub banner: String,
}

/// Information about an existing shortcut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutInfo {
    pub app_id: u32,
    pub name: String,
    pub exe: String,
    pub start_dir: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub launch_options: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_played: i64,
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

/// Current state of an upload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UploadStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

/// Progress information for an active upload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProgress {
    pub upload_id: String,
    pub status: UploadStatus,
    pub total_bytes: i64,
    pub transferred_bytes: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_file: String,
    pub started_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
}

impl UploadProgress {
    /// Returns the upload progress as a percentage (0-100).
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.transferred_bytes as f64 / self.total_bytes as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_info_json_roundtrip() {
        let info = AgentInfo {
            id: "test-id".into(),
            name: "Test Agent".into(),
            platform: "linux".into(),
            version: "0.6.0".into(),
            accept_connections: true,
            supported_image_formats: vec!["png".into(), "jpg".into()],
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: AgentInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, parsed);
    }

    #[test]
    fn agent_info_field_names() {
        let json = r#"{"id":"a","name":"b","platform":"c","version":"d","acceptConnections":true,"supportedImageFormats":["png"]}"#;
        let info: AgentInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.id, "a");
        assert!(info.accept_connections);
    }

    #[test]
    fn shortcut_info_omit_empty() {
        let info = ShortcutInfo {
            app_id: 123,
            name: "Test".into(),
            exe: "/bin/test".into(),
            start_dir: "/home".into(),
            launch_options: String::new(),
            tags: vec![],
            last_played: 0,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("launchOptions"));
        assert!(!json.contains("tags"));
        assert!(!json.contains("lastPlayed"));
    }

    #[test]
    fn upload_status_serialization() {
        assert_eq!(
            serde_json::to_string(&UploadStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&UploadStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );
    }

    #[test]
    fn upload_progress_percentage() {
        let p = UploadProgress {
            upload_id: "u1".into(),
            status: UploadStatus::InProgress,
            total_bytes: 1000,
            transferred_bytes: 500,
            current_file: String::new(),
            started_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:01:00Z".into(),
            error: String::new(),
        };
        assert!((p.percentage() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn upload_progress_percentage_zero_total() {
        let p = UploadProgress {
            upload_id: "u1".into(),
            status: UploadStatus::Pending,
            total_bytes: 0,
            transferred_bytes: 0,
            current_file: String::new(),
            started_at: String::new(),
            updated_at: String::new(),
            error: String::new(),
        };
        assert!((p.percentage() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn upload_config_omit_empty() {
        let cfg = UploadConfig {
            game_name: "Game".into(),
            install_path: "/games".into(),
            executable: "game.exe".into(),
            launch_options: String::new(),
            tags: String::new(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(!json.contains("launchOptions"));
        assert!(!json.contains("tags"));
    }
}
