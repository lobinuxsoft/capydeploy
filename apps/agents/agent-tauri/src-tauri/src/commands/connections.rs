use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::State;

use crate::state::AgentState;

#[tauri::command]
pub async fn set_accept_connections(
    accept: bool,
    state: State<'_, Arc<AgentState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state.accept_connections.store(accept, Ordering::Relaxed);
    tracing::info!("Accept connections: {accept}");

    if !accept {
        // Disconnect current Hub — same cleanup as disconnect_hub.
        *state.connected_hub.lock().await = None;

        let sender = state.hub_sender.lock().unwrap().take();
        if let Some(ws) = &sender {
            ws.disconnect();
        }

        state.telemetry_collector.stop().await;
        state.console_log_collector.stop().await;
    }

    // TODO: toggle mDNS advertisement

    super::emit_status(&app, &state).await;
    Ok(())
}

#[tauri::command]
pub async fn disconnect_hub(
    state: State<'_, Arc<AgentState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Immediate cleanup matching Go agent's DisconnectHub().
    // on_hub_disconnected will run later but find everything already clean.
    *state.connected_hub.lock().await = None;

    let sender = state.hub_sender.lock().unwrap().take();
    if let Some(ws) = &sender {
        ws.disconnect();
    }

    state.telemetry_collector.stop().await;
    state.console_log_collector.stop().await;

    tracing::info!("Hub disconnected (local)");
    super::emit_status(&app, &state).await;
    Ok(())
}
