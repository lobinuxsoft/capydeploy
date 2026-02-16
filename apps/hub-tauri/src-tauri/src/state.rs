use std::sync::Arc;

use tokio::sync::Mutex;

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
}
