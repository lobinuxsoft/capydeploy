use std::sync::Arc;

use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::state::AgentState;

#[tauri::command]
pub async fn select_install_path(
    state: State<'_, Arc<AgentState>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let path = app
        .dialog()
        .file()
        .set_title("Select Install Path")
        .blocking_pick_folder();

    let path = match path {
        Some(p) => p.to_string(),
        None => return Ok(String::new()), // User cancelled
    };

    // Save the selected path
    let mut config = state.config.lock().await;
    config.install_path = path.clone();
    config.save().map_err(|e| e.to_string())?;
    drop(config);

    tracing::info!("Install path changed to: {path}");
    super::emit_status(&app, &state).await;
    Ok(path)
}
