//! Console log Tauri commands.

use tauri::State;

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::messages::{SetConsoleLogEnabledRequest, SetConsoleLogFilterRequest};

use crate::state::HubState;

#[tauri::command]
pub async fn set_console_log_filter(
    state: State<'_, HubState>,
    level_mask: u32,
) -> Result<(), String> {
    let payload = SetConsoleLogFilterRequest { level_mask };
    state
        .connection_mgr
        .send_request(MessageType::SetConsoleLogFilter, Some(&payload))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_console_log_enabled(
    state: State<'_, HubState>,
    enabled: bool,
) -> Result<(), String> {
    let payload = SetConsoleLogEnabledRequest { enabled };
    state
        .connection_mgr
        .send_request(MessageType::SetConsoleLogEnabled, Some(&payload))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
