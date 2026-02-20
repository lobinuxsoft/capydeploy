//! Deploy-related Tauri commands (game setup CRUD + upload).

use tauri::{AppHandle, Emitter, State};

use capydeploy_hub_deploy::GameSetup;

use crate::agent_adapter::DeployAdapter;
use crate::state::HubState;
use crate::types::UploadProgressDto;

#[tauri::command]
pub async fn get_game_setups(state: State<'_, HubState>) -> Result<Vec<GameSetup>, String> {
    let cfg = state.config.lock().await;
    Ok(cfg.game_setups.clone())
}

#[tauri::command]
pub async fn add_game_setup(
    state: State<'_, HubState>,
    mut setup: GameSetup,
) -> Result<(), String> {
    if setup.id.is_empty() {
        setup.id = uuid::Uuid::new_v4().to_string();
    }
    let mut cfg = state.config.lock().await;
    cfg.game_setups.push(setup);
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_game_setup(
    state: State<'_, HubState>,
    id: String,
    setup: GameSetup,
) -> Result<(), String> {
    let mut cfg = state.config.lock().await;
    if let Some(existing) = cfg.game_setups.iter_mut().find(|s| s.id == id) {
        *existing = setup;
        cfg.save().map_err(|e| e.to_string())
    } else {
        Err(format!("game setup '{id}' not found"))
    }
}

#[tauri::command]
pub async fn remove_game_setup(state: State<'_, HubState>, id: String) -> Result<(), String> {
    let mut cfg = state.config.lock().await;
    let len_before = cfg.game_setups.len();
    cfg.game_setups.retain(|s| s.id != id);
    if cfg.game_setups.len() == len_before {
        return Err(format!("game setup '{id}' not found"));
    }
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upload_game(
    app: AppHandle,
    state: State<'_, HubState>,
    id: String,
) -> Result<(), String> {
    let cfg = state.config.lock().await;
    let setup = cfg
        .game_setups
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or_else(|| format!("game setup '{id}' not found"))?;
    drop(cfg);

    let connected = state
        .connection_mgr
        .get_connected()
        .await
        .ok_or_else(|| "not connected to any agent".to_string())?;

    let mgr = state.connection_mgr.clone();
    let agent_id = connected.agent.info.id.clone();
    let adapter = DeployAdapter::with_agent_info(mgr, agent_id, &connected);

    let artwork = capydeploy_hub_deploy::build_artwork_assignment(&setup);
    let deploy_config = capydeploy_hub_deploy::DeployConfig { setup, artwork };

    let mut orchestrator = capydeploy_hub_deploy::DeployOrchestrator::new();
    let events_rx = orchestrator.take_events();

    // Spawn event forwarder — we keep the JoinHandle so we can await
    // it after deploy() to guarantee the Completed/Failed event reaches
    // the frontend before the command returns.
    let app_clone = app.clone();
    let forwarder = events_rx.map(|mut rx| {
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let dto = match event {
                    capydeploy_hub_deploy::DeployEvent::Progress {
                        progress, status, ..
                    } => UploadProgressDto {
                        progress,
                        status,
                        error: None,
                        done: false,
                    },
                    capydeploy_hub_deploy::DeployEvent::Completed { .. } => UploadProgressDto {
                        progress: 1.0,
                        status: "completed".into(),
                        error: None,
                        done: true,
                    },
                    capydeploy_hub_deploy::DeployEvent::Failed { error, .. } => UploadProgressDto {
                        progress: 0.0,
                        status: "failed".into(),
                        error: Some(error),
                        done: true,
                    },
                };
                let _ = app_clone.emit("upload:progress", &dto);
            }
        })
    });

    let results = orchestrator.deploy(deploy_config, vec![&adapter]).await;

    // Drop the orchestrator (and its events_tx sender) so the forwarder
    // channel closes and the task can drain remaining events.
    drop(orchestrator);
    if let Some(handle) = forwarder {
        let _ = handle.await;
    }

    if let Some(result) = results.first()
        && !result.success
    {
        return Err(result
            .error
            .clone()
            .unwrap_or_else(|| "upload failed".into()));
    }

    Ok(())
}
