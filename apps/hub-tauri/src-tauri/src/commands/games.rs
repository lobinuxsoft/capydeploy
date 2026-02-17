//! Installed games Tauri commands.

use tauri::State;

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::messages::ConfigResponse;

use crate::agent_adapter::GamesAdapter;
use crate::state::HubState;
use crate::types::InstalledGameDto;

#[tauri::command]
pub async fn get_installed_games(
    state: State<'_, HubState>,
    _agent_id: String,
) -> Result<Vec<InstalledGameDto>, String> {
    let connected = state
        .connection_mgr
        .get_connected()
        .await
        .ok_or_else(|| "not connected".to_string())?;

    let mgr = state.connection_mgr.clone();
    let agent_id = connected.agent.info.id.clone();
    let adapter = GamesAdapter::new(mgr, agent_id);

    let games_mgr = capydeploy_hub_games::GamesManager::new(reqwest::Client::new());
    let games = games_mgr
        .get_installed_games(&adapter)
        .await
        .map_err(|e| e.to_string())?;

    Ok(games
        .into_iter()
        .map(|g| InstalledGameDto {
            name: g.name,
            path: g.path,
            size: g.size,
            app_id: if g.app_id == 0 { None } else { Some(g.app_id) },
        })
        .collect())
}

#[tauri::command]
pub async fn delete_game(
    state: State<'_, HubState>,
    _agent_id: String,
    app_id: u32,
) -> Result<(), String> {
    let connected = state
        .connection_mgr
        .get_connected()
        .await
        .ok_or_else(|| "not connected".to_string())?;

    let mgr = state.connection_mgr.clone();
    let agent_id = connected.agent.info.id.clone();
    let adapter = GamesAdapter::new(mgr, agent_id);

    let games_mgr = capydeploy_hub_games::GamesManager::new(reqwest::Client::new());
    games_mgr
        .delete_game(&adapter, app_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_game_artwork(
    state: State<'_, HubState>,
    app_id: u32,
    grid: String,
    hero: String,
    logo: String,
    icon: String,
    _game_id: i32,
) -> Result<(), String> {
    let connected = state
        .connection_mgr
        .get_connected()
        .await
        .ok_or_else(|| "not connected".to_string())?;

    let mgr = state.connection_mgr.clone();
    let agent_id = connected.agent.info.id.clone();
    let adapter = GamesAdapter::new(mgr, agent_id);

    let artwork = capydeploy_hub_games::ArtworkUpdate {
        grid,
        banner: String::new(),
        hero,
        logo,
        icon,
    };

    let games_mgr = capydeploy_hub_games::GamesManager::new(reqwest::Client::new());
    games_mgr
        .update_game_artwork(&adapter, app_id, &artwork)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_agent_install_path(state: State<'_, HubState>) -> Result<String, String> {
    let mgr = state.connection_mgr.clone();
    let resp = mgr
        .send_request::<()>(MessageType::GetConfig, None)
        .await
        .map_err(|e| e.to_string())?;

    let config: ConfigResponse = resp
        .parse_payload::<ConfigResponse>()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "empty config response".to_string())?;

    Ok(config.install_path)
}

#[tauri::command]
pub async fn set_game_log_wrapper(
    state: State<'_, HubState>,
    app_id: u32,
    enabled: bool,
) -> Result<(), String> {
    let connected = state
        .connection_mgr
        .get_connected()
        .await
        .ok_or_else(|| "not connected".to_string())?;

    let mgr = state.connection_mgr.clone();
    let agent_id = connected.agent.info.id.clone();
    let adapter = GamesAdapter::new(mgr, agent_id);

    let games_mgr = capydeploy_hub_games::GamesManager::new(reqwest::Client::new());
    games_mgr
        .set_game_log_wrapper(&adapter, app_id, enabled)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
