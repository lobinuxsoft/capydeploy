use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::State;

use crate::state::AgentState;

#[tauri::command]
pub async fn set_telemetry_enabled(
    enabled: bool,
    state: State<'_, Arc<AgentState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state.telemetry_enabled.store(enabled, Ordering::Relaxed);

    let mut config = state.config.lock().await;
    config.telemetry_enabled = enabled;
    let interval = config.telemetry_interval;
    config.save().map_err(|e| e.to_string())?;
    drop(config);

    // Start/stop collector
    if enabled {
        state.telemetry_collector.start(interval as u32).await;
    } else {
        state.telemetry_collector.stop().await;
    }

    tracing::info!("Telemetry enabled: {enabled}");
    super::emit_status(&app, &state).await;
    Ok(())
}

#[tauri::command]
pub async fn set_telemetry_interval(
    seconds: i32,
    state: State<'_, Arc<AgentState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let seconds = seconds.clamp(1, 10);

    let mut config = state.config.lock().await;
    config.telemetry_interval = seconds;
    config.save().map_err(|e| e.to_string())?;
    drop(config);

    // Update collector interval if running
    if state.telemetry_enabled.load(Ordering::Relaxed) {
        state
            .telemetry_collector
            .update_interval(seconds as u32)
            .await;
    }

    tracing::info!("Telemetry interval changed to: {seconds}s");
    super::emit_status(&app, &state).await;
    Ok(())
}
