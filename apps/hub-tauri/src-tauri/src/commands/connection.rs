//! Connection-related Tauri commands.

use tauri::State;
use tracing::{debug, warn};

use crate::state::HubState;
use crate::types::{ConnectionStatusDto, DiscoveredAgentDto};

#[tauri::command]
pub async fn get_discovered_agents(
    state: State<'_, HubState>,
) -> Result<Vec<DiscoveredAgentDto>, String> {
    let agents = state.connection_mgr.get_discovered().await;
    Ok(agents.iter().map(DiscoveredAgentDto::from).collect())
}

#[tauri::command]
pub async fn refresh_discovery(
    state: State<'_, HubState>,
) -> Result<Vec<DiscoveredAgentDto>, String> {
    state.connection_mgr.refresh_discovery().await;
    let agents = state.connection_mgr.get_discovered().await;
    Ok(agents.iter().map(DiscoveredAgentDto::from).collect())
}

#[tauri::command]
pub async fn connect_agent(
    state: State<'_, HubState>,
    agent_id: String,
) -> Result<String, String> {
    // Returns "connected" or "pairing_required" so the frontend can
    // distinguish both cases and avoid false "Connected" toasts.
    match state.connection_mgr.connect_agent(&agent_id).await {
        Ok(_) => Ok("connected".into()),
        Err(capydeploy_hub_connection::WsError::PairingFailed(_)) => Ok("pairing_required".into()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn disconnect_agent(state: State<'_, HubState>) -> Result<(), String> {
    state.connection_mgr.disconnect_agent().await;
    Ok(())
}

#[tauri::command]
pub async fn get_connection_status(
    state: State<'_, HubState>,
) -> Result<ConnectionStatusDto, String> {
    match state.connection_mgr.get_connected().await {
        Some(agent) => Ok(ConnectionStatusDto::from_connected(&agent)),
        None => Ok(ConnectionStatusDto::disconnected()),
    }
}

#[tauri::command]
pub async fn confirm_pairing(
    state: State<'_, HubState>,
    agent_id: String,
    code: String,
) -> Result<(), String> {
    debug!(agent_id = %agent_id, code_len = code.len(), "confirm_pairing called");
    match state.connection_mgr.confirm_pairing(&agent_id, &code).await {
        Ok(_) => {
            debug!(agent_id = %agent_id, "pairing confirmed successfully");
            Ok(())
        }
        Err(e) => {
            warn!(agent_id = %agent_id, error = %e, "pairing confirmation failed");
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn cancel_pairing(state: State<'_, HubState>) -> Result<(), String> {
    state.connection_mgr.disconnect_agent().await;
    Ok(())
}
