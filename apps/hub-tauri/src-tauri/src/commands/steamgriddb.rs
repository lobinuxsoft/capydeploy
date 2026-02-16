//! SteamGridDB Tauri commands.

use tauri::State;

use capydeploy_steamgriddb::{Client as SgdbClient, ImageData, SearchResult};

use crate::state::HubState;
use crate::types::ImageFiltersDto;

/// Creates a SteamGridDB client from the current API key.
async fn get_client(state: &State<'_, HubState>) -> Result<SgdbClient, String> {
    let cfg = state.config.lock().await;
    if cfg.steamgriddb_api_key.is_empty() {
        return Err("SteamGridDB API key not set".into());
    }
    SgdbClient::new(&cfg.steamgriddb_api_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_games(
    state: State<'_, HubState>,
    query: String,
) -> Result<Vec<SearchResult>, String> {
    let client = get_client(&state).await?;
    client.search(&query).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_grids(
    state: State<'_, HubState>,
    game_id: i32,
    filters: ImageFiltersDto,
    page: i32,
) -> Result<Vec<ImageData>, String> {
    let client = get_client(&state).await?;
    let f = capydeploy_steamgriddb::ImageFilters::from(filters);
    client
        .get_grids(game_id, Some(&f), page)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_heroes(
    state: State<'_, HubState>,
    game_id: i32,
    filters: ImageFiltersDto,
    page: i32,
) -> Result<Vec<ImageData>, String> {
    let client = get_client(&state).await?;
    let f = capydeploy_steamgriddb::ImageFilters::from(filters);
    client
        .get_heroes(game_id, Some(&f), page)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_logos(
    state: State<'_, HubState>,
    game_id: i32,
    filters: ImageFiltersDto,
    page: i32,
) -> Result<Vec<ImageData>, String> {
    let client = get_client(&state).await?;
    let f = capydeploy_steamgriddb::ImageFilters::from(filters);
    client
        .get_logos(game_id, Some(&f), page)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_icons(
    state: State<'_, HubState>,
    game_id: i32,
    filters: ImageFiltersDto,
    page: i32,
) -> Result<Vec<ImageData>, String> {
    let client = get_client(&state).await?;
    let f = capydeploy_steamgriddb::ImageFilters::from(filters);
    client
        .get_icons(game_id, Some(&f), page)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_artwork_preview(
    state: State<'_, HubState>,
    url: String,
) -> Result<String, String> {
    let client = get_client(&state).await?;
    let data = client
        .download_image(&url)
        .await
        .map_err(|e| e.to_string())?;

    // Detect content type from URL extension.
    let content_type = if url.ends_with(".png") {
        "image/png"
    } else if url.ends_with(".webp") {
        "image/webp"
    } else if url.ends_with(".ico") {
        "image/vnd.microsoft.icon"
    } else {
        "image/jpeg"
    };

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    Ok(format!("data:{content_type};base64,{b64}"))
}
