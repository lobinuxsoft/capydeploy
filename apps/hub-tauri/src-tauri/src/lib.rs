mod agent_adapter;
mod commands;
mod config;
mod events;
mod state;
mod types;

use std::sync::Arc;

use tracing_subscriber::EnvFilter;

use capydeploy_hub_connection::ConnectionManager;
use capydeploy_hub_connection::pairing::TokenStore;
use capydeploy_hub_telemetry::TelemetryHub;

use config::HubConfig;
use state::HubState;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,capydeploy=debug")),
        )
        .init();

    let cfg = HubConfig::load().unwrap_or_default();

    let identity = capydeploy_hub_connection::HubIdentity {
        name: cfg.name.clone(),
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        hub_id: cfg.hub_id.clone(),
    };

    // Use the same token path as the Go Hub: ~/.config/capydeploy-hub/tokens.json
    let token_store = config::token_store_path()
        .and_then(|path| {
            TokenStore::new(path)
                .map_err(|e| tracing::warn!("failed to load token store: {e}"))
                .ok()
        })
        .map(Arc::new);

    let mgr = Arc::new(ConnectionManager::new(identity, token_store));
    let mgr_shutdown = mgr.clone();

    let hub_state = HubState {
        connection_mgr: mgr.clone(),
        telemetry_hub: Arc::new(tokio::sync::Mutex::new(TelemetryHub::new())),
        console_hub: Arc::new(tokio::sync::Mutex::new(
            capydeploy_hub_console_log::ConsoleLogHub::new(),
        )),
        config: Arc::new(tokio::sync::Mutex::new(cfg)),
    };

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(hub_state)
        .setup(move |app| {
            let handle = app.handle().clone();
            let mgr_clone = mgr.clone();
            tauri::async_runtime::spawn(async move {
                events::event_loop(handle, mgr_clone).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Connection
            commands::connection::get_discovered_agents,
            commands::connection::refresh_discovery,
            commands::connection::connect_agent,
            commands::connection::disconnect_agent,
            commands::connection::get_connection_status,
            commands::connection::confirm_pairing,
            commands::connection::cancel_pairing,
            // Settings
            commands::settings::get_version,
            commands::settings::get_hub_info,
            commands::settings::get_hub_name,
            commands::settings::set_hub_name,
            commands::settings::get_steamgriddb_api_key,
            commands::settings::set_steamgriddb_api_key,
            commands::settings::get_cache_size,
            commands::settings::clear_image_cache,
            commands::settings::open_cache_folder,
            commands::settings::get_image_cache_enabled,
            commands::settings::set_image_cache_enabled,
            commands::settings::get_game_log_directory,
            commands::settings::set_game_log_directory,
            // Deploy
            commands::deploy::get_game_setups,
            commands::deploy::add_game_setup,
            commands::deploy::update_game_setup,
            commands::deploy::remove_game_setup,
            commands::deploy::upload_game,
            // Games
            commands::games::get_installed_games,
            commands::games::delete_game,
            commands::games::update_game_artwork,
            commands::games::set_game_log_wrapper,
            // SteamGridDB
            commands::steamgriddb::search_games,
            commands::steamgriddb::get_grids,
            commands::steamgriddb::get_heroes,
            commands::steamgriddb::get_logos,
            commands::steamgriddb::get_icons,
            commands::steamgriddb::get_artwork_preview,
            // Console log
            commands::console_log::set_console_log_filter,
            commands::console_log::set_console_log_enabled,
            // File dialogs
            commands::files::select_folder,
            commands::files::select_artwork_file,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(move |_handle, event| {
        if let tauri::RunEvent::Exit = event {
            tracing::info!("shutting down hub — cleaning up connections");
            tauri::async_runtime::block_on(mgr_shutdown.shutdown());
        }
    });
}
