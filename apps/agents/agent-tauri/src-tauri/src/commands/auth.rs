use std::sync::Arc;

use tauri::{Emitter, State};

use crate::state::AgentState;
use crate::types::AuthorizedHubDto;

#[tauri::command]
pub async fn get_authorized_hubs(
    state: State<'_, Arc<AgentState>>,
) -> Result<Vec<AuthorizedHubDto>, String> {
    let config = state.config.lock().await;
    Ok(config
        .authorized_hubs
        .iter()
        .map(|h| AuthorizedHubDto {
            id: h.id.clone(),
            name: h.name.clone(),
            paired_at: h.paired_at.clone(),
            last_seen: h.last_seen.clone(),
        })
        .collect())
}

#[tauri::command]
pub async fn revoke_hub(
    hub_id: String,
    state: State<'_, Arc<AgentState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.remove_authorized_hub(&hub_id);
    config.save().map_err(|e| e.to_string())?;
    drop(config);

    tracing::info!("Revoked Hub: {hub_id}");

    // Immediate disconnect if the revoked Hub is currently connected.
    // Matches Go agent: cleanup state first, then close WS.
    // on_hub_disconnected will run later but find everything already clean.
    {
        let mut hub = state.connected_hub.lock().await;
        if hub.as_ref().is_some_and(|h| h.id == hub_id) {
            *hub = None;
            drop(hub);

            let sender = state.hub_sender.lock().unwrap().take();
            if let Some(ws) = &sender {
                ws.disconnect();
            }

            state.telemetry_collector.stop().await;
            state.console_log_collector.stop().await;
        }
    }

    let _ = app.emit("auth:hub-revoked", &hub_id);
    super::emit_status(&app, &state).await;
    Ok(())
}
