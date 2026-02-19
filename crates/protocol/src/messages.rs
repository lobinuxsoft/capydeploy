use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::{AgentInfo, ArtworkConfig, ShortcutConfig, ShortcutInfo, UploadConfig};

// ---------------------------------------------------------------------------
// Request payloads
// ---------------------------------------------------------------------------

/// Starts a new upload session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitUploadRequest {
    pub config: UploadConfig,
    pub total_size: i64,
    pub file_count: i32,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub resume_from: i64,
}

/// Sends a chunk of upload data.
///
/// The `data` field is base64-encoded in JSON to match Go's `[]byte` serialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadChunkRequest {
    pub upload_id: String,
    pub offset: i64,
    #[serde(with = "base64_bytes")]
    pub data: Vec<u8>,
    pub file_path: String,
    pub is_last: bool,
}

/// Finalizes an upload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteUploadRequest {
    pub upload_id: String,
    pub create_shortcut: bool,
}

/// Cancels an active upload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelUploadRequest {
    pub upload_id: String,
}

/// Sets which log levels the agent should collect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetConsoleLogFilterRequest {
    pub level_mask: u32,
}

/// Enables or disables console log streaming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetConsoleLogEnabledRequest {
    pub enabled: bool,
}

/// Creates a Steam shortcut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateShortcutRequest {
    pub user_id: u32,
    pub shortcut: ShortcutConfig,
}

/// Removes a Steam shortcut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteShortcutRequest {
    pub user_id: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub app_id: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// Lists shortcuts for a user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListShortcutsRequest {
    pub user_id: u32,
}

/// Requests artwork application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyArtworkRequest {
    pub user_id: String,
    pub app_id: u32,
    pub artwork: Option<ArtworkConfig>,
}

/// Requests deletion of a game (agent handles everything internally).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteGameRequest {
    pub app_id: u32,
}

/// Enables or disables the game log wrapper for a specific game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetGameLogWrapperRequest {
    pub app_id: u32,
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Connection payloads
// ---------------------------------------------------------------------------

/// Sent when a Hub connects to an Agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubConnectedRequest {
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub platform: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hub_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub token: String,
    /// Protocol version advertised by the Hub (0 = legacy/pre-negotiation).
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub protocol_version: u32,
}

/// Agent's response to a Hub connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusResponse {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub accept_connections: bool,
    pub telemetry_enabled: bool,
    pub telemetry_interval: i32,
    pub console_log_enabled: bool,
    /// Protocol version advertised by the Agent (0 = legacy/pre-negotiation).
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub protocol_version: u32,
}

/// Sent when a Hub needs to pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingRequiredResponse {
    pub code: String,
    pub expires_in: i32,
}

/// Sent by Hub to confirm pairing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairConfirmRequest {
    pub code: String,
}

/// Sent when pairing is successful.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairSuccessResponse {
    pub token: String,
}

/// Sent when pairing fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairFailedResponse {
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Response payloads
// ---------------------------------------------------------------------------

/// Confirms the applied log level mask.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetConsoleLogFilterResponse {
    pub level_mask: u32,
}

/// Confirms the console log enabled state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetConsoleLogEnabledResponse {
    pub enabled: bool,
}

/// Contains agent information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfoResponse {
    pub agent: AgentInfo,
}

/// Acknowledges upload initialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitUploadResponse {
    pub upload_id: String,
    pub resume_from: i64,
}

/// Acknowledges a chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadChunkResponse {
    pub upload_id: String,
    pub bytes_written: i64,
    pub total_written: i64,
}

/// Confirms upload completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteUploadResponse {
    pub upload_id: String,
    pub success: bool,
}

/// Contains shortcut operation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortcutResponse {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shortcuts: Vec<ShortcutInfo>,
}

/// Contains Steam status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SteamStatusResponse {
    pub running: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
}

/// Contains error details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub details: String,
}

/// Agent configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    pub install_path: String,
}

// ---------------------------------------------------------------------------
// Steam payloads
// ---------------------------------------------------------------------------

/// A Steam user (matches `steam.User` structure).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamUser {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub avatar_url: String,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_login_at: i64,
}

/// List of Steam users.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SteamUsersResponse {
    pub users: Vec<SteamUser>,
}

/// List of shortcuts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortcutsListResponse {
    pub shortcuts: Vec<ShortcutInfo>,
}

/// Result of shortcut creation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateShortcutResponse {
    pub app_id: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub steam_restarted: bool,
}

/// Shortcut deletion with restart option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteShortcutWithRestartRequest {
    pub user_id: String,
    pub app_id: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub restart_steam: bool,
}

/// Result of game deletion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteGameResponse {
    pub status: String,
    pub game_name: String,
    pub steam_restarted: bool,
}

// ---------------------------------------------------------------------------
// Artwork payloads
// ---------------------------------------------------------------------------

/// Artwork operation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtworkResponse {
    pub applied: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<ArtworkFailed>,
}

/// A failed artwork application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtworkFailed {
    #[serde(rename = "type")]
    pub art_type: String,
    pub error: String,
}

/// Binary artwork image transfer acknowledgment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkImageResponse {
    pub success: bool,
    pub artwork_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
}

// ---------------------------------------------------------------------------
// Operation payloads
// ---------------------------------------------------------------------------

/// Generic result for operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationResult {
    pub success: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
}

/// Push notification for operation progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub status: String,
    pub game_name: String,
    pub progress: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
}

// ---------------------------------------------------------------------------
// Steam control payloads
// ---------------------------------------------------------------------------

/// Result of Steam restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestartSteamResponse {
    pub success: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Upload extended payloads
// ---------------------------------------------------------------------------

/// Upload request with file manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitUploadRequestFull {
    pub config: UploadConfig,
    pub total_size: i64,
    pub files: Vec<FileEntry>,
}

/// A file in the upload manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub relative_path: String,
    pub size: i64,
}

/// Upload response with chunk size configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitUploadResponseFull {
    pub upload_id: String,
    pub chunk_size: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_from: Option<HashMap<String, i64>>,
}

/// Upload chunk with full metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadChunkRequestFull {
    pub upload_id: String,
    pub file_path: String,
    pub offset: i64,
    pub size: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub checksum: String,
}

/// Upload completion with shortcut configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteUploadRequestFull {
    pub upload_id: String,
    pub create_shortcut: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<ShortcutConfig>,
}

/// Upload completion result with path and app ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteUploadResponseFull {
    pub success: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub app_id: u32,
}

/// Upload progress event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProgressEvent {
    pub upload_id: String,
    pub transferred_bytes: i64,
    pub total_bytes: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_file: String,
    pub percentage: f64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

fn is_false(v: &bool) -> bool {
    !v
}

/// Custom base64 serde module matching Go's automatic `[]byte` ↔ base64 encoding.
mod base64_bytes {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        STANDARD.encode(data).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_chunk_base64_roundtrip() {
        let req = UploadChunkRequest {
            upload_id: "u1".into(),
            offset: 0,
            data: vec![0x48, 0x65, 0x6c, 0x6c, 0x6f],
            file_path: "test.bin".into(),
            is_last: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        // Go encodes []byte as base64, "Hello" = "SGVsbG8="
        assert!(json.contains("SGVsbG8="));
        let parsed: UploadChunkRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.data, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn hub_connected_omit_empty() {
        let req = HubConnectedRequest {
            name: "Hub".into(),
            version: "0.1.0".into(),
            platform: String::new(),
            hub_id: String::new(),
            token: String::new(),
            protocol_version: 0,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("platform"));
        assert!(!json.contains("hubId"));
        assert!(!json.contains("token"));
        assert!(!json.contains("protocolVersion"));
    }

    #[test]
    fn hub_connected_protocol_version_present() {
        let req = HubConnectedRequest {
            name: "Hub".into(),
            version: "0.1.0".into(),
            platform: String::new(),
            hub_id: String::new(),
            token: String::new(),
            protocol_version: 1,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"protocolVersion\":1"));
    }

    #[test]
    fn hub_connected_legacy_json_defaults_to_zero() {
        let json = r#"{"name":"Hub","version":"0.1.0"}"#;
        let req: HubConnectedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.protocol_version, 0);
    }

    #[test]
    fn agent_status_response_roundtrip() {
        let resp = AgentStatusResponse {
            name: "Agent".into(),
            version: "0.6.0".into(),
            platform: "steamdeck".into(),
            accept_connections: true,
            telemetry_enabled: true,
            telemetry_interval: 5,
            console_log_enabled: false,
            protocol_version: 1,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"acceptConnections\":true"));
        assert!(json.contains("\"protocolVersion\":1"));
        let parsed: AgentStatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, parsed);
    }

    #[test]
    fn agent_status_legacy_json_defaults_to_zero() {
        let json = r#"{
            "name":"Agent","version":"0.6.0","platform":"steamdeck",
            "acceptConnections":true,"telemetryEnabled":true,
            "telemetryInterval":5,"consoleLogEnabled":false
        }"#;
        let resp: AgentStatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.protocol_version, 0);
    }

    #[test]
    fn init_upload_response_full_roundtrip() {
        let mut resume = HashMap::new();
        resume.insert("file1.bin".to_string(), 1024i64);
        let resp = InitUploadResponseFull {
            upload_id: "u1".into(),
            chunk_size: 1048576,
            resume_from: Some(resume),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: InitUploadResponseFull = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, parsed);
    }

    #[test]
    fn operation_event_roundtrip() {
        let evt = OperationEvent {
            event_type: "install".into(),
            status: "progress".into(),
            game_name: "TestGame".into(),
            progress: 42.5,
            message: String::new(),
        };
        let json = serde_json::to_string(&evt).unwrap();
        // Go uses `json:"type"`, Rust uses `#[serde(rename = "type")]`
        assert!(json.contains("\"type\":\"install\""));
        assert!(!json.contains("message"));
        let parsed: OperationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(evt, parsed);
    }

    #[test]
    fn delete_shortcut_omit_empty() {
        let req = DeleteShortcutRequest {
            user_id: 12345,
            app_id: 0,
            name: String::new(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("appId"));
        assert!(!json.contains("name"));
    }

    #[test]
    fn artwork_failed_type_field() {
        let f = ArtworkFailed {
            art_type: "hero".into(),
            error: "file not found".into(),
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains("\"type\":\"hero\""));
    }

    #[test]
    fn game_log_wrapper_status_map_keys() {
        // Go serializes map[uint32]bool with string keys
        let json = r#"{"wrappers":{"12345":true,"67890":false}}"#;
        let parsed: super::super::telemetry::GameLogWrapperStatusEvent =
            serde_json::from_str(json).unwrap();
        assert_eq!(parsed.wrappers.get(&12345), Some(&true));
        assert_eq!(parsed.wrappers.get(&67890), Some(&false));
    }
}
