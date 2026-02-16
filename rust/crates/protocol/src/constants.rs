use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Time allowed to write a WebSocket message.
pub const WS_WRITE_WAIT: Duration = Duration::from_secs(30);

/// Time to wait for a pong response (or any incoming message).
///
/// This acts as a read deadline: if *nothing* arrives within this window
/// (no pong, no response, no push event), the connection is considered
/// dead. Set high enough to tolerate slow chunk processing on the agent
/// side during large file transfers.
pub const WS_PONG_WAIT: Duration = Duration::from_secs(60);

/// How often to send pings (must be < agent's pong wait of 15s).
pub const WS_PING_PERIOD: Duration = Duration::from_secs(5);

/// Maximum message size in bytes (50 MB).
pub const WS_MAX_MESSAGE_SIZE: usize = 50 * 1024 * 1024;

/// Size for binary chunks (1 MB).
pub const WS_CHUNK_SIZE: usize = 1024 * 1024;

/// Timeout for request/response operations (text messages).
pub const WS_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for binary request/response operations (chunk uploads).
///
/// Binary transfers (chunk uploads, artwork images) may take significantly
/// longer than text requests due to disk I/O and network conditions.
pub const WS_BINARY_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// WebSocket message type identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    // Connection management
    #[serde(rename = "hub_connected")]
    HubConnected,
    #[serde(rename = "agent_status")]
    AgentStatus,

    // Authentication / Pairing
    #[serde(rename = "pairing_required")]
    PairingRequired,
    #[serde(rename = "pair_confirm")]
    PairConfirm,
    #[serde(rename = "pair_success")]
    PairSuccess,
    #[serde(rename = "pair_failed")]
    PairFailed,

    // Requests from Hub to Agent
    #[serde(rename = "set_console_log_filter")]
    SetConsoleLogFilter,
    #[serde(rename = "set_console_log_enabled")]
    SetConsoleLogEnabled,
    #[serde(rename = "set_game_log_wrapper")]
    SetGameLogWrapper,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "get_info")]
    GetInfo,
    #[serde(rename = "get_config")]
    GetConfig,
    #[serde(rename = "get_steam_users")]
    GetSteamUsers,
    #[serde(rename = "list_shortcuts")]
    ListShortcuts,
    #[serde(rename = "create_shortcut")]
    CreateShortcut,
    #[serde(rename = "delete_shortcut")]
    DeleteShortcut,
    #[serde(rename = "delete_game")]
    DeleteGame,
    #[serde(rename = "apply_artwork")]
    ApplyArtwork,
    #[serde(rename = "send_artwork_image")]
    SendArtworkImage,
    #[serde(rename = "restart_steam")]
    RestartSteam,
    #[serde(rename = "init_upload")]
    InitUpload,
    #[serde(rename = "upload_chunk")]
    UploadChunk,
    #[serde(rename = "complete_upload")]
    CompleteUpload,
    #[serde(rename = "cancel_upload")]
    CancelUpload,

    // Responses from Agent to Hub
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "info_response")]
    InfoResponse,
    #[serde(rename = "config_response")]
    ConfigResponse,
    #[serde(rename = "steam_users_response")]
    SteamUsersResponse,
    #[serde(rename = "shortcuts_response")]
    ShortcutsResponse,
    #[serde(rename = "artwork_response")]
    ArtworkResponse,
    #[serde(rename = "artwork_image_response")]
    ArtworkImageResponse,
    #[serde(rename = "steam_response")]
    SteamResponse,
    #[serde(rename = "upload_init_response")]
    UploadInitResponse,
    #[serde(rename = "upload_chunk_response")]
    UploadChunkResponse,
    #[serde(rename = "operation_result")]
    OperationResult,
    #[serde(rename = "error")]
    Error,

    // Events from Agent to Hub (push notifications)
    #[serde(rename = "upload_progress")]
    UploadProgress,
    #[serde(rename = "operation_event")]
    OperationEvent,
    #[serde(rename = "telemetry_status")]
    TelemetryStatus,
    #[serde(rename = "telemetry_data")]
    TelemetryData,
    #[serde(rename = "console_log_status")]
    ConsoleLogStatus,
    #[serde(rename = "console_log_data")]
    ConsoleLogData,
    #[serde(rename = "game_log_wrapper_status")]
    GameLogWrapperStatus,

    /// Forward compatibility: unknown message types deserialize here.
    #[serde(other)]
    Unknown,
}

/// Common WebSocket error codes.
pub const WS_ERR_CODE_BAD_REQUEST: i32 = 400;
pub const WS_ERR_CODE_UNAUTHORIZED: i32 = 401;
pub const WS_ERR_CODE_NOT_FOUND: i32 = 404;
pub const WS_ERR_CODE_NOT_ACCEPTED: i32 = 406;
pub const WS_ERR_CODE_CONFLICT: i32 = 409;
pub const WS_ERR_CODE_INTERNAL: i32 = 500;
pub const WS_ERR_CODE_NOT_IMPLEMENTED: i32 = 501;

/// Console log level bitmask: `log` messages.
pub const LOG_LEVEL_LOG: u32 = 1;
/// Console log level bitmask: `warn` messages.
pub const LOG_LEVEL_WARN: u32 = 2;
/// Console log level bitmask: `error` messages.
pub const LOG_LEVEL_ERROR: u32 = 4;
/// Console log level bitmask: `info` messages.
pub const LOG_LEVEL_INFO: u32 = 8;
/// Console log level bitmask: `debug` messages.
pub const LOG_LEVEL_DEBUG: u32 = 16;
/// Default log level mask: Log + Warn + Error + Info (debug off).
pub const LOG_LEVEL_DEFAULT: u32 =
    LOG_LEVEL_LOG | LOG_LEVEL_WARN | LOG_LEVEL_ERROR | LOG_LEVEL_INFO;

/// Maps a CDP log level string to its bitmask bit. Returns 0 for unknown levels.
pub fn log_level_bit(level: &str) -> u32 {
    match level {
        "log" => LOG_LEVEL_LOG,
        "warn" | "warning" => LOG_LEVEL_WARN,
        "error" => LOG_LEVEL_ERROR,
        "info" => LOG_LEVEL_INFO,
        "debug" | "verbose" => LOG_LEVEL_DEBUG,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_type_serialization() {
        assert_eq!(
            serde_json::to_string(&MessageType::HubConnected).unwrap(),
            "\"hub_connected\""
        );
        assert_eq!(
            serde_json::to_string(&MessageType::AgentStatus).unwrap(),
            "\"agent_status\""
        );
        assert_eq!(
            serde_json::to_string(&MessageType::UploadProgress).unwrap(),
            "\"upload_progress\""
        );
    }

    #[test]
    fn message_type_deserialization() {
        let mt: MessageType = serde_json::from_str("\"hub_connected\"").unwrap();
        assert_eq!(mt, MessageType::HubConnected);
    }

    #[test]
    fn unknown_message_type() {
        let mt: MessageType = serde_json::from_str("\"some_future_type\"").unwrap();
        assert_eq!(mt, MessageType::Unknown);
    }

    #[test]
    fn log_level_bit_mapping() {
        assert_eq!(log_level_bit("log"), 1);
        assert_eq!(log_level_bit("warn"), 2);
        assert_eq!(log_level_bit("warning"), 2);
        assert_eq!(log_level_bit("error"), 4);
        assert_eq!(log_level_bit("info"), 8);
        assert_eq!(log_level_bit("debug"), 16);
        assert_eq!(log_level_bit("verbose"), 16);
        assert_eq!(log_level_bit("unknown"), 0);
    }

    #[test]
    fn log_level_default() {
        assert_eq!(LOG_LEVEL_DEFAULT, 15);
    }
}
