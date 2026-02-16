use std::collections::HashSet;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::state::AgentState;
use crate::types::{ShortcutDto, SteamUserDto};

#[tauri::command]
pub async fn get_steam_users() -> Result<Vec<SteamUserDto>, String> {
    let users = capydeploy_steam::get_users().map_err(|e| e.to_string())?;
    Ok(users
        .into_iter()
        .map(|u| SteamUserDto {
            id: u.id.clone(),
            name: u.id,
        })
        .collect())
}

#[tauri::command]
pub async fn get_shortcuts(
    user_id: String,
    state: State<'_, Arc<AgentState>>,
) -> Result<Vec<ShortcutDto>, String> {
    let sm = capydeploy_steam::ShortcutManager::new().map_err(|e| e.to_string())?;
    let vdf_path = sm.shortcuts_path(&user_id);

    // Read VDF shortcuts, filtering out app IDs that were deleted via CEF
    // but not yet flushed from the VDF file.
    let deleted = state.deleted_app_ids.lock().await;
    let vdf_list =
        capydeploy_steam::load_shortcuts_vdf(std::path::Path::new(&vdf_path)).unwrap_or_default();

    let mut seen_ids: HashSet<u32> = HashSet::new();
    let mut result: Vec<ShortcutDto> = vdf_list
        .into_iter()
        .filter(|s| !deleted.contains(&s.app_id))
        .map(|s| {
            seen_ids.insert(s.app_id);
            ShortcutDto {
                app_id: s.app_id,
                name: s.name,
                exe: s.exe,
                start_dir: s.start_dir,
            }
        })
        .collect();
    drop(deleted);

    // Merge tracked shortcuts not yet in VDF (CEF-created, not flushed yet).
    let tracked = state.tracked_shortcuts.lock().await;
    for ts in tracked.iter() {
        if !seen_ids.contains(&ts.app_id) {
            result.push(ShortcutDto {
                app_id: ts.app_id,
                name: ts.name.clone(),
                exe: ts.exe.clone(),
                start_dir: ts.start_dir.clone(),
            });
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn delete_shortcut(
    user_id: String,
    app_id: u32,
    state: State<'_, Arc<AgentState>>,
    app: AppHandle,
) -> Result<(), String> {
    // Ensure CEF debug file exists.
    let ctrl = capydeploy_steam::Controller::new();
    let _ = ctrl.ensure_cef_debug_file();

    // Remove via CEF API.
    let cef = capydeploy_steam::CefClient::new();
    cef.remove_shortcut(app_id)
        .await
        .map_err(|e| format!("failed to remove shortcut via CEF: {e}"))?;

    // Remove from tracked list and mark as deleted.
    {
        let mut tracked = state.tracked_shortcuts.lock().await;
        tracked.retain(|s| s.app_id != app_id);
    }
    {
        let mut deleted = state.deleted_app_ids.lock().await;
        deleted.insert(app_id);
    }

    // Delete artwork files (best-effort).
    if let Ok(sm) = capydeploy_steam::ShortcutManager::new() {
        let _ = sm.delete_artwork(&user_id, app_id);
    }

    let _ = app.emit("shortcuts:changed", &());

    Ok(())
}
