use std::sync::Arc;

use tauri::State;

use crate::state::AgentState;

#[tauri::command]
pub async fn set_name(
    name: String,
    state: State<'_, Arc<AgentState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if name.is_empty() {
        return Err("name cannot be empty".into());
    }

    let mut config = state.config.lock().await;
    config.name = name.clone();
    config.save().map_err(|e| e.to_string())?;
    drop(config);

    tracing::info!("Agent name changed to: {name}");

    // TODO: restart mDNS discovery with new name
    super::emit_status(&app, &state).await;
    Ok(())
}

#[tauri::command]
pub async fn get_install_path(state: State<'_, Arc<AgentState>>) -> Result<String, String> {
    let config = state.config.lock().await;
    Ok(config.install_path.clone())
}

#[tauri::command]
pub async fn set_install_path(
    path: String,
    state: State<'_, Arc<AgentState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".into());
    }

    let mut config = state.config.lock().await;
    config.install_path = path.clone();
    config.save().map_err(|e| e.to_string())?;
    drop(config);

    tracing::info!("Install path changed to: {path}");
    super::emit_status(&app, &state).await;
    Ok(())
}
