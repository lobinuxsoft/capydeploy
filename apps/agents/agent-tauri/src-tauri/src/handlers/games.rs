use tauri::Emitter;

use capydeploy_agent_server::Sender;
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::handler::TauriAgentHandler;
use crate::helpers::{delete_game_directory, expand_path};

impl TauriAgentHandler {
    pub(crate) async fn handle_get_steam_users(&self, sender: Sender, msg: Message) {
        match capydeploy_steam::get_users() {
            Ok(users) => {
                let proto_users: Vec<messages::SteamUser> = users
                    .iter()
                    .map(|u| messages::SteamUser {
                        id: u.id.clone(),
                        name: String::new(),
                        avatar_url: String::new(),
                        last_login_at: 0,
                    })
                    .collect();
                let resp = messages::SteamUsersResponse { users: proto_users };
                if let Ok(reply) = msg.reply(MessageType::SteamUsersResponse, Some(&resp)) {
                    let _ = sender.send_msg(reply);
                }
            }
            Err(e) => {
                tracing::error!("failed to get Steam users: {e}");
                let _ = sender.send_error(&msg, 500, &e.to_string());
            }
        }
    }

    pub(crate) async fn handle_delete_game(&self, sender: Sender, msg: Message) {
        let req: messages::DeleteGameRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ = sender.send_error(&msg, 400, "invalid payload");
                return;
            }
        };

        // Get first Steam user (same as Go agent: automatic, Hub only sends appId).
        let users = match capydeploy_steam::get_users() {
            Ok(u) if !u.is_empty() => u,
            Ok(_) => {
                let _ = sender.send_error(&msg, 500, "no Steam users found");
                return;
            }
            Err(e) => {
                let _ = sender.send_error(&msg, 500, &format!("failed to get Steam users: {e}"));
                return;
            }
        };
        let user_id = &users[0].id;

        // Load shortcuts from VDF to find game name and directory.
        let sm = match capydeploy_steam::ShortcutManager::new() {
            Ok(sm) => sm,
            Err(e) => {
                let _ =
                    sender.send_error(&msg, 500, &format!("failed to init ShortcutManager: {e}"));
                return;
            }
        };

        let vdf_path = sm.shortcuts_path(user_id);
        let shortcuts = capydeploy_steam::load_shortcuts_vdf(std::path::Path::new(&vdf_path))
            .unwrap_or_default();

        let shortcut = shortcuts.iter().find(|sc| sc.app_id == req.app_id);
        let (game_name, game_dir) = match shortcut {
            Some(sc) => {
                // VDF may store paths with surrounding quotes.
                let dir = sc.start_dir.trim_matches('"').to_string();
                (sc.name.clone(), dir)
            }
            None => {
                // Shortcut may exist only in tracked list (CEF not flushed to VDF yet).
                let tracked = self.state.tracked_shortcuts.lock().await;
                match tracked.iter().find(|ts| ts.app_id == req.app_id) {
                    Some(ts) => {
                        let dir = ts.start_dir.trim_matches('"').to_string();
                        (ts.name.clone(), dir)
                    }
                    None => {
                        let _ = sender.send_error(&msg, 404, "game not found");
                        return;
                    }
                }
            }
        };

        // Notify Hub + local UI about delete start.
        self.emit_operation(&sender, "delete", "start", &game_name, 0.0, "Eliminando...");

        // Ensure CEF debug file exists (Steam reads it at startup).
        let ctrl = capydeploy_steam::Controller::new();
        if let Err(e) = ctrl.ensure_cef_debug_file() {
            tracing::warn!("failed to ensure CEF debug file: {e}");
        }

        // Remove shortcut via Steam CEF API (instant, no restart needed).
        self.emit_operation(
            &sender,
            "delete",
            "progress",
            &game_name,
            20.0,
            "Eliminando shortcut...",
        );
        let cef_timeout = std::time::Duration::from_secs(15);
        let cef_result = tokio::time::timeout(cef_timeout, async {
            let cef_client = capydeploy_steam::CefClient::new();
            cef_client.remove_shortcut(req.app_id).await
        })
        .await;
        match cef_result {
            Ok(Ok(())) => {
                tracing::info!("removed shortcut via CEF for AppID {}", req.app_id);
            }
            Ok(Err(e)) => {
                tracing::warn!("CEF remove_shortcut failed: {e} — continuing cleanup");
            }
            Err(_) => {
                tracing::warn!("CEF remove_shortcut timed out (15s) — continuing cleanup");
            }
        }

        // Remove from tracked shortcuts list and mark as deleted.
        {
            let mut tracked = self.state.tracked_shortcuts.lock().await;
            tracked.retain(|s| s.app_id != req.app_id);
        }
        {
            let mut deleted = self.state.deleted_app_ids.lock().await;
            deleted.insert(req.app_id);
        }

        // Delete game directory (best-effort with safety checks).
        self.emit_operation(
            &sender,
            "delete",
            "progress",
            &game_name,
            50.0,
            "Eliminando archivos...",
        );
        if !game_dir.is_empty() {
            let expanded = expand_path(&game_dir);
            if let Err(e) = delete_game_directory(&expanded) {
                tracing::warn!("failed to delete game directory: {e}");
            }
        }

        // Delete artwork files from Steam grid folder (best-effort).
        self.emit_operation(
            &sender,
            "delete",
            "progress",
            &game_name,
            80.0,
            "Eliminando artwork...",
        );
        if let Err(e) = sm.delete_artwork(user_id, req.app_id) {
            tracing::warn!("failed to delete artwork: {e}");
        }

        tracing::info!(
            "Deleted game '{}' (AppID: {}) for user {}",
            game_name,
            req.app_id,
            user_id
        );

        // Notify completion.
        self.emit_operation(
            &sender,
            "delete",
            "complete",
            &game_name,
            100.0,
            "Eliminado",
        );

        // Refresh shortcuts list in the local agent UI.
        let _ = self.app_handle.emit("shortcuts:changed", &());

        let resp = messages::DeleteGameResponse {
            status: "deleted".into(),
            game_name,
            steam_restarted: false,
        };
        if let Ok(reply) = msg.reply(MessageType::OperationResult, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }
    }

    pub(crate) async fn handle_restart_steam(&self, sender: Sender, msg: Message) {
        let ctrl = capydeploy_steam::Controller::new();
        let result = ctrl.restart().await;
        let resp = messages::RestartSteamResponse {
            success: result.success,
            message: result.message,
        };
        if let Ok(reply) = msg.reply(MessageType::SteamResponse, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }
    }
}
