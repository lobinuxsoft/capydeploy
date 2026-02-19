use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::auth::AuthManager;
use crate::config::AgentConfig;

/// Shared application state managed by Tauri.
pub struct AgentState {
    pub config: Arc<Mutex<AgentConfig>>,
    pub auth: Arc<Mutex<AuthManager>>,
    pub accept_connections: Arc<AtomicBool>,
    pub connected_hub: Arc<Mutex<Option<ConnectedHubInfo>>>,
    pub server_port: Arc<Mutex<u16>>,
    pub uploads: Arc<Mutex<HashMap<String, UploadSession>>>,
    pub pending_artwork: Arc<Mutex<Vec<PendingArtwork>>>,
    pub telemetry_enabled: Arc<AtomicBool>,
    pub console_log_enabled: Arc<AtomicBool>,
    /// WS sender to the connected Hub (sync Mutex for use in sync callbacks).
    pub hub_sender: Arc<std::sync::Mutex<Option<capydeploy_agent_server::Sender>>>,
    pub telemetry_collector: Arc<capydeploy_telemetry::Collector>,
    pub console_log_collector: Arc<capydeploy_console_log::Collector>,
    /// Wrapper manager for game log injection (Linux only).
    #[cfg(target_os = "linux")]
    pub game_log_wrapper: Arc<capydeploy_game_log::WrapperManager>,
    /// Log tailer for game output (Linux only).
    #[cfg(target_os = "linux")]
    pub game_log_tailer: Arc<capydeploy_game_log::LogTailer>,
    /// Tracked shortcuts created/managed by this agent (mirrors Go agent's Manager.tracked).
    /// Used for listing since CEF-created shortcuts may not be flushed to VDF immediately.
    pub tracked_shortcuts: Arc<Mutex<Vec<TrackedShortcut>>>,
    /// App IDs deleted via CEF but possibly still in VDF (not flushed yet).
    /// Filtered out in get_shortcuts to avoid showing stale entries.
    pub deleted_app_ids: Arc<Mutex<HashSet<u32>>>,
    /// Cancellation token for graceful shutdown of all background tasks.
    pub shutdown_token: CancellationToken,
}

/// A shortcut tracked by the agent (created via CEF).
#[derive(Debug, Clone)]
pub struct TrackedShortcut {
    pub app_id: u32,
    pub name: String,
    pub exe: String,
    pub start_dir: String,
}

/// Info about the currently connected Hub.
#[derive(Debug, Clone)]
pub struct ConnectedHubInfo {
    pub id: String,
    pub name: String,
    pub ip: String,
}

/// An active upload session.
#[allow(dead_code)]
pub struct UploadSession {
    pub id: String,
    pub game_name: String,
    pub install_path: String,
    pub executable: String,
    pub total_size: i64,
    pub transferred: i64,
    pub current_file: String,
    pub active: bool,
}

impl UploadSession {
    pub fn percentage(&self) -> f64 {
        if self.total_size == 0 {
            return 0.0;
        }
        self.transferred as f64 / self.total_size as f64 * 100.0
    }
}

/// Artwork data buffered until shortcut creation provides a real AppID.
pub struct PendingArtwork {
    pub artwork_type: String,
    pub content_type: String,
    pub data: Vec<u8>,
}
