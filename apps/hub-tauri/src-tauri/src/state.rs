use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use capydeploy_hub_connection::ConnectionManager;
use capydeploy_hub_console_log::ConsoleLogHub;
use capydeploy_hub_telemetry::TelemetryHub;

use crate::config::HubConfig;

/// Shared application state managed by Tauri.
pub struct HubState {
    pub connection_mgr: Arc<ConnectionManager>,
    pub telemetry_hub: Arc<Mutex<TelemetryHub>>,
    pub console_hub: Arc<Mutex<ConsoleLogHub>>,
    pub config: Arc<Mutex<HubConfig>>,
    /// Active deploy cancellation token (set during upload, cleared after).
    pub deploy_cancel: Arc<Mutex<Option<CancellationToken>>>,
}
