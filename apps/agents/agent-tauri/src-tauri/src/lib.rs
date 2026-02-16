mod auth;
mod commands;
mod config;
mod events;
mod handler;
mod state;
mod types;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use config::AgentConfig;
use state::AgentState;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,capydeploy=debug")),
        )
        .init();

    let cfg = AgentConfig::load().unwrap_or_default();
    let agent_name = cfg.name.clone();

    // Shared WS sender for forwarding telemetry/console-log to Hub
    let hub_sender: Arc<std::sync::Mutex<Option<capydeploy_agent_server::Sender>>> =
        Arc::new(std::sync::Mutex::new(None));

    // Telemetry collector — callback forwards TelemetryData to Hub if connected
    let telem_ws = hub_sender.clone();
    let telemetry_collector = Arc::new(capydeploy_telemetry::Collector::new(Box::new(
        move |data| {
            let sender = telem_ws.lock().unwrap();
            if let Some(ws) = sender.as_ref() {
                let id = uuid::Uuid::new_v4().to_string();
                if let Ok(msg) = Message::new(id, MessageType::TelemetryData, Some(&data)) {
                    if let Err(e) = ws.send_msg(msg) {
                        tracing::warn!("telemetry send failed: {e}");
                    }
                }
            }
        },
    )));

    // Console log collector — callback forwards ConsoleLogBatch to Hub if connected
    let cl_ws = hub_sender.clone();
    let console_log_collector = Arc::new(capydeploy_console_log::Collector::new(Box::new(
        move |batch| {
            let sender = cl_ws.lock().unwrap();
            if let Some(ws) = sender.as_ref() {
                let id = uuid::Uuid::new_v4().to_string();
                if let Ok(msg) = Message::new(id, MessageType::ConsoleLogData, Some(&batch)) {
                    if let Err(e) = ws.send_msg(msg) {
                        tracing::warn!("console log send failed: {e}");
                    }
                }
            }
        },
    )));

    let shutdown_token = CancellationToken::new();

    let agent_state = AgentState {
        accept_connections: Arc::new(AtomicBool::new(true)),
        telemetry_enabled: Arc::new(AtomicBool::new(cfg.telemetry_enabled)),
        console_log_enabled: Arc::new(AtomicBool::new(cfg.console_log_enabled)),
        connected_hub: Arc::new(tokio::sync::Mutex::new(None)),
        server_port: Arc::new(tokio::sync::Mutex::new(0)),
        uploads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        pending_artwork: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        auth: Arc::new(tokio::sync::Mutex::new(auth::AuthManager::new())),
        config: Arc::new(tokio::sync::Mutex::new(cfg)),
        hub_sender,
        telemetry_collector,
        console_log_collector,
        tracked_shortcuts: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        deleted_app_ids: Arc::new(tokio::sync::Mutex::new(std::collections::HashSet::new())),
        shutdown_token: shutdown_token.clone(),
    };

    let state_arc = Arc::new(agent_state);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state_arc.clone())
        .setup(move |app| {
            // System tray
            let show = MenuItemBuilder::with_id("show", "Show Window").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app).item(&show).separator().item(&quit).build()?;

            TrayIconBuilder::new()
                .tooltip(&format!("CapyDeploy Agent — {agent_name}"))
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => {
                            tracing::info!("Quit requested from tray");
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Start WS server + discovery
            let handle = app.handle().clone();
            let state = state_arc.clone();
            tauri::async_runtime::spawn(async move {
                events::start_server(handle, state).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Status
            commands::status::get_status,
            commands::status::get_version,
            // Settings
            commands::settings::set_name,
            commands::settings::get_install_path,
            commands::settings::set_install_path,
            // Connections
            commands::connections::set_accept_connections,
            commands::connections::disconnect_hub,
            // Steam
            commands::steam::get_steam_users,
            commands::steam::get_shortcuts,
            commands::steam::delete_shortcut,
            // Telemetry
            commands::telemetry::set_telemetry_enabled,
            commands::telemetry::set_telemetry_interval,
            // Console log
            commands::console_log::set_console_log_enabled,
            // Auth
            commands::auth::get_authorized_hubs,
            commands::auth::revoke_hub,
            // Files
            commands::files::select_install_path,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(move |_handle, event| {
        if let tauri::RunEvent::Exit = event {
            tracing::info!("shutting down agent — cleaning up server and collectors");
            shutdown_token.cancel();
        }
    });
}
