use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::State;

use crate::state::AgentState;

#[tauri::command]
pub async fn set_console_log_enabled(
    enabled: bool,
    state: State<'_, Arc<AgentState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state
        .console_log_enabled
        .store(enabled, Ordering::Relaxed);

    // Start/stop collector
    if enabled {
        state.console_log_collector.start().await;
    } else {
        state.console_log_collector.stop().await;
    }

    let mut config = state.config.lock().await;
    config.console_log_enabled = enabled;
    config.save().map_err(|e| e.to_string())?;
    drop(config);

    tracing::info!("Console log enabled: {enabled}");
    super::emit_status(&app, &state).await;
    Ok(())
}
