//! Settings and hub info Tauri commands.

use std::path::PathBuf;

use tauri::State;

use crate::state::HubState;
use crate::types::{HubInfoDto, VersionInfoDto};

#[tauri::command]
pub async fn get_version() -> Result<VersionInfoDto, String> {
    Ok(VersionInfoDto {
        version: env!("CAPYDEPLOY_VERSION").into(),
        commit: option_env!("CAPYDEPLOY_COMMIT").unwrap_or("dev").into(),
        build_date: option_env!("CAPYDEPLOY_BUILD_DATE").unwrap_or("").into(),
    })
}

#[tauri::command]
pub async fn get_hub_info(state: State<'_, HubState>) -> Result<HubInfoDto, String> {
    let cfg = state.config.lock().await;
    Ok(HubInfoDto {
        id: cfg.hub_id.clone(),
        name: cfg.name.clone(),
        platform: std::env::consts::OS.into(),
    })
}

#[tauri::command]
pub async fn get_hub_name(state: State<'_, HubState>) -> Result<String, String> {
    let cfg = state.config.lock().await;
    Ok(cfg.name.clone())
}

#[tauri::command]
pub async fn set_hub_name(state: State<'_, HubState>, name: String) -> Result<(), String> {
    let mut cfg = state.config.lock().await;
    cfg.name = name;
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_steamgriddb_api_key(state: State<'_, HubState>) -> Result<String, String> {
    let cfg = state.config.lock().await;
    Ok(cfg.steamgriddb_api_key.clone())
}

#[tauri::command]
pub async fn set_steamgriddb_api_key(
    state: State<'_, HubState>,
    key: String,
) -> Result<(), String> {
    let mut cfg = state.config.lock().await;
    cfg.steamgriddb_api_key = key;
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_cache_size() -> Result<u64, String> {
    let cache_dir = image_cache_dir();
    if !cache_dir.exists() {
        return Ok(0);
    }
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

#[tauri::command]
pub async fn clear_image_cache() -> Result<(), String> {
    let cache_dir = image_cache_dir();
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn open_cache_folder() -> Result<(), String> {
    let cache_dir = image_cache_dir();
    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    }
    open::that(&cache_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_image_cache_enabled() -> Result<bool, String> {
    // Always enabled for now — matches Go behavior.
    Ok(true)
}

#[tauri::command]
pub async fn set_image_cache_enabled(_enabled: bool) -> Result<(), String> {
    // No-op for now.
    Ok(())
}

#[tauri::command]
pub async fn get_game_log_directory(state: State<'_, HubState>) -> Result<String, String> {
    let cfg = state.config.lock().await;
    Ok(cfg.game_log_dir.clone())
}

#[tauri::command]
pub async fn set_game_log_directory(
    state: State<'_, HubState>,
    path: String,
) -> Result<(), String> {
    let mut cfg = state.config.lock().await;
    cfg.game_log_dir = path;
    cfg.save().map_err(|e| e.to_string())
}

fn image_cache_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home)
            .join(".cache")
            .join("capydeploy")
            .join("images")
    }
    #[cfg(target_os = "windows")]
    {
        let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Temp".into());
        PathBuf::from(local)
            .join("capydeploy")
            .join("cache")
            .join("images")
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        PathBuf::from("/tmp")
            .join("capydeploy-cache")
            .join("images")
    }
}
