//! Handler implementation for the agent WS server.
//!
//! `TauriAgentHandler` implements the `agent_server::Handler` trait,
//! dispatching all Hub→Agent messages.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter};

use capydeploy_agent_server::{
    BinaryArtworkHeader, BinaryChunkHeader, Handler, HandlerFuture, Sender,
};
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::auth::AuthManager;
use crate::config::AuthorizedHub;
use crate::state::{AgentState, ConnectedHubInfo, PendingArtwork, TrackedShortcut, UploadSession};
use crate::types::ConnectedHubDto;

pub struct TauriAgentHandler {
    pub app_handle: AppHandle,
    pub state: Arc<AgentState>,
}

// Helper: get local IPs (non-loopback IPv4)
fn local_ips() -> Vec<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| {
            if iface.is_loopback() {
                return None;
            }
            match iface.addr.ip() {
                std::net::IpAddr::V4(ip) => {
                    // Skip link-local (169.254.x.x)
                    if ip.octets()[0] == 169 && ip.octets()[1] == 254 {
                        return None;
                    }
                    Some(ip.to_string())
                }
                _ => None,
            }
        })
        .collect()
}

/// Parses an artwork type string ("grid", "hero", etc.) into the crate enum.
fn parse_artwork_type(s: &str) -> Option<capydeploy_steam::ArtworkType> {
    match s {
        "grid" => Some(capydeploy_steam::ArtworkType::Grid),
        "hero" => Some(capydeploy_steam::ArtworkType::Hero),
        "logo" => Some(capydeploy_steam::ArtworkType::Logo),
        "icon" => Some(capydeploy_steam::ArtworkType::Icon),
        "portrait" | "banner" => Some(capydeploy_steam::ArtworkType::Portrait),
        _ => None,
    }
}

/// Extracts a file extension from an image MIME type.
fn ext_from_content_type(ct: &str) -> &str {
    match ct {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/x-icon" | "image/vnd.microsoft.icon" => "ico",
        _ => "png",
    }
}

impl Handler for TauriAgentHandler {
    // -----------------------------------------------------------------------
    // Connection + Pairing
    // -----------------------------------------------------------------------

    fn on_hub_connected(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::HubConnectedRequest = match msg.parse_payload() {
                Ok(Some(r)) => r,
                _ => {
                    let _ = sender.send_error(&msg, 400, "invalid payload");
                    return;
                }
            };

            tracing::info!(
                "Hub connected: {} v{} (ID: {})",
                req.name,
                req.version,
                req.hub_id
            );

            // If Hub provided a token, validate it
            if !req.token.is_empty() && !req.hub_id.is_empty() {
                let config = self.state.config.lock().await;
                if AuthManager::validate_token(&config.authorized_hubs, &req.hub_id, &req.token) {
                    tracing::info!("Hub {} authenticated with valid token", req.name);
                    // Update last_seen
                    drop(config);
                    let mut config = self.state.config.lock().await;
                    config.update_hub_last_seen(&req.hub_id, &chrono::Utc::now().to_rfc3339());
                    let _ = config.save();
                    drop(config);

                    self.accept_hub(&sender, &msg, &req).await;
                    return;
                }
                tracing::info!("Hub {} provided invalid token, requiring pairing", req.name);
            }

            // Hub not authorized — require pairing
            if req.hub_id.is_empty() {
                let _ = sender.send_error(&msg, 401, "hub_id required for pairing");
                return;
            }

            // Generate pairing code
            let mut auth = self.state.auth.lock().await;
            match auth.generate_code(&req.hub_id, &req.name, &req.platform) {
                Ok(code) => {
                    tracing::info!("Pairing required for Hub {}, code: {}", req.name, code);

                    // Emit pairing code to local UI
                    let _ = self.app_handle.emit("pairing:code", &code);

                    // Send pairing_required to Hub
                    let resp = messages::PairingRequiredResponse {
                        code,
                        expires_in: 60,
                    };
                    if let Ok(reply) = msg.reply(MessageType::PairingRequired, Some(&resp)) {
                        let _ = sender.send_msg(reply);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to generate pairing code: {e}");
                    let _ = sender.send_error(&msg, 500, &e.to_string());
                }
            }
        })
    }

    fn on_pair_confirm(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::PairConfirmRequest = match msg.parse_payload() {
                Ok(Some(r)) => r,
                _ => {
                    let _ = sender.send_error(&msg, 400, "invalid payload");
                    return;
                }
            };

            // Need Hub ID from the pending session
            let mut auth = self.state.auth.lock().await;
            let session = match auth.pending_pairing() {
                Some(s) => s.clone(),
                None => {
                    let resp = messages::PairFailedResponse {
                        reason: "no pending pairing".into(),
                    };
                    if let Ok(reply) = msg.reply(MessageType::PairFailed, Some(&resp)) {
                        let _ = sender.send_msg(reply);
                    }
                    return;
                }
            };

            match auth.validate_code(&session.hub_id, &session.hub_name, &req.code) {
                Ok(token) => {
                    tracing::info!("Pairing successful for Hub {}", session.hub_name);

                    // Store authorized Hub in config
                    let mut config = self.state.config.lock().await;
                    let now = chrono::Utc::now().to_rfc3339();
                    config.add_authorized_hub(AuthorizedHub {
                        id: session.hub_id.clone(),
                        name: session.hub_name.clone(),
                        platform: session.hub_platform.clone(),
                        token: token.clone(),
                        paired_at: now.clone(),
                        last_seen: now,
                    });
                    let _ = config.save();
                    drop(config);

                    // Set hub_sender so data forwarding works during
                    // the pairing window (before Hub reconnects with token).
                    *self.state.hub_sender.lock().unwrap() = Some(sender.clone());

                    // Send pair_success with token
                    let resp = messages::PairSuccessResponse { token };
                    if let Ok(reply) = msg.reply(MessageType::PairSuccess, Some(&resp)) {
                        let _ = sender.send_msg(reply);
                    }

                    // Emit pairing success to local UI
                    let _ = self.app_handle.emit("pairing:success", &());
                    let _ = self.app_handle.emit("hubs:changed", &());

                    // Update connected Hub state
                    *self.state.connected_hub.lock().await = Some(ConnectedHubInfo {
                        id: session.hub_id,
                        name: session.hub_name,
                        ip: String::new(),
                    });

                    self.emit_status_changed().await;
                }
                Err(e) => {
                    tracing::warn!("Pairing failed: {e}");
                    let resp = messages::PairFailedResponse {
                        reason: e.to_string(),
                    };
                    if let Ok(reply) = msg.reply(MessageType::PairFailed, Some(&resp)) {
                        let _ = sender.send_msg(reply);
                    }
                }
            }
        })
    }

    // -----------------------------------------------------------------------
    // Info + Config
    // -----------------------------------------------------------------------

    fn on_get_info(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let config = self.state.config.lock().await;
            let info = capydeploy_protocol::types::AgentInfo {
                id: generate_agent_id(&config.name),
                name: config.name.clone(),
                platform: std::env::consts::OS.into(),
                version: env!("CARGO_PKG_VERSION").into(),
                accept_connections: self.state.accept_connections.load(Ordering::Relaxed),
                supported_image_formats: vec![
                    "png".into(),
                    "jpg".into(),
                    "jpeg".into(),
                    "webp".into(),
                ],
            };
            let resp = messages::InfoResponse { agent: info };
            if let Ok(reply) = msg.reply(MessageType::InfoResponse, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    fn on_get_config(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let config = self.state.config.lock().await;
            let resp = messages::ConfigResponse {
                install_path: config.install_path.clone(),
            };
            if let Ok(reply) = msg.reply(MessageType::ConfigResponse, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    // -----------------------------------------------------------------------
    // Steam operations
    // -----------------------------------------------------------------------

    fn on_get_steam_users(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
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
        })
    }

    fn on_list_shortcuts(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::ListShortcutsRequest = match msg.parse_payload() {
                Ok(Some(r)) => r,
                _ => {
                    let _ = sender.send_error(&msg, 400, "invalid payload");
                    return;
                }
            };

            let user_id = req.user_id.to_string();
            let list = match capydeploy_steam::ShortcutManager::new() {
                Ok(sm) => {
                    let vdf_path = sm.shortcuts_path(&user_id);
                    capydeploy_steam::load_shortcuts_vdf(std::path::Path::new(&vdf_path))
                        .unwrap_or_default()
                }
                Err(e) => {
                    tracing::warn!("failed to init ShortcutManager: {e}");
                    vec![]
                }
            };

            let resp = messages::ShortcutsListResponse { shortcuts: list };
            if let Ok(reply) = msg.reply(MessageType::ShortcutsResponse, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    fn on_create_shortcut(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            // TODO: implement VDF write for shortcut creation
            let _ = sender.send_error(&msg, 501, "shortcut creation not yet implemented");
        })
    }

    fn on_delete_shortcut(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            // TODO: implement VDF write for shortcut deletion
            let _ = sender.send_error(&msg, 501, "shortcut deletion not yet implemented");
        })
    }

    fn on_delete_game(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
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
                    let _ =
                        sender.send_error(&msg, 500, &format!("failed to get Steam users: {e}"));
                    return;
                }
            };
            let user_id = &users[0].id;

            // Load shortcuts from VDF to find game name and directory.
            let sm = match capydeploy_steam::ShortcutManager::new() {
                Ok(sm) => sm,
                Err(e) => {
                    let _ = sender.send_error(
                        &msg,
                        500,
                        &format!("failed to init ShortcutManager: {e}"),
                    );
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
        })
    }

    fn on_apply_artwork(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            // TODO: implement URL-based artwork download + apply
            let _ = sender.send_error(&msg, 501, "apply_artwork not yet implemented");
        })
    }

    fn on_restart_steam(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let ctrl = capydeploy_steam::Controller::new();
            let result = ctrl.restart().await;
            let resp = messages::RestartSteamResponse {
                success: result.success,
                message: result.message,
            };
            if let Ok(reply) = msg.reply(MessageType::SteamResponse, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    // -----------------------------------------------------------------------
    // Upload pipeline
    // -----------------------------------------------------------------------

    fn on_init_upload(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::InitUploadRequestFull = match msg.parse_payload() {
                Ok(Some(r)) => r,
                _ => {
                    let _ = sender.send_error(&msg, 400, "invalid payload");
                    return;
                }
            };

            if req.config.game_name.is_empty() {
                let _ = sender.send_error(&msg, 400, "gameName is required");
                return;
            }

            let upload_id = uuid::Uuid::new_v4().to_string();
            let session = UploadSession {
                id: upload_id.clone(),
                game_name: req.config.game_name.clone(),
                install_path: req.config.install_path.clone(),
                executable: req.config.executable.clone(),
                total_size: req.total_size,
                transferred: 0,
                current_file: String::new(),
                active: true,
            };

            self.state
                .uploads
                .lock()
                .await
                .insert(upload_id.clone(), session);

            tracing::info!(
                "Upload session created: {} for game '{}' ({} bytes, {} files)",
                upload_id,
                req.config.game_name,
                req.total_size,
                req.files.len()
            );

            self.emit_operation(
                &sender,
                "install",
                "start",
                &req.config.game_name,
                0.0,
                "Iniciando instalación...",
            );

            let resp = messages::InitUploadResponseFull {
                upload_id,
                chunk_size: 1_048_576, // 1MB
                resume_from: None,
            };
            if let Ok(reply) = msg.reply(MessageType::UploadInitResponse, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    fn on_upload_chunk(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            // Handled by binary path — JSON chunks are a fallback
            let _ = sender.send_error(&msg, 501, "use binary chunk protocol");
        })
    }

    fn on_binary_chunk(
        &self,
        sender: Sender,
        header: BinaryChunkHeader,
        data: Vec<u8>,
    ) -> HandlerFuture<'_> {
        Box::pin(async move {
            let mut uploads = self.state.uploads.lock().await;
            let session = match uploads.get_mut(&header.upload_id) {
                Some(s) if s.active => s,
                _ => {
                    tracing::warn!(
                        "binary chunk for unknown/inactive upload: {}",
                        header.upload_id
                    );
                    return;
                }
            };

            // Build game path
            let config = self.state.config.lock().await;
            let base_path = expand_path(&config.install_path);
            drop(config);
            let game_path = PathBuf::from(&base_path).join(&session.game_name);

            // Write chunk to disk using transfer crate
            let chunk = capydeploy_transfer::Chunk {
                offset: header.offset,
                size: data.len(),
                data,
                file_path: header.file_path.clone(),
                checksum: header.checksum.clone(),
            };

            let mut writer = capydeploy_transfer::ChunkWriter::new(&game_path);
            if let Err(e) = writer.write_chunk(&chunk) {
                session.active = false;
                tracing::error!("failed to write chunk: {e}");
                return;
            }

            let chunk_len = chunk.size as i64;
            session.transferred += chunk_len;
            session.current_file = header.file_path.clone();
            let percentage = session.percentage();
            let upload_id = session.id.clone();
            let total = session.total_size;
            let transferred = session.transferred;
            let game_name = session.game_name.clone();
            drop(uploads);

            // Emit progress
            let progress_evt = messages::UploadProgressEvent {
                upload_id: upload_id.clone(),
                transferred_bytes: transferred,
                total_bytes: total,
                current_file: header.file_path.clone(),
                percentage,
            };
            self.send_event(&sender, MessageType::UploadProgress, &progress_evt);
            self.emit_operation(&sender, "install", "progress", &game_name, percentage, "");

            // Send chunk ack
            let resp = messages::UploadChunkResponse {
                upload_id,
                bytes_written: chunk_len,
                total_written: transferred,
            };
            if let Ok(reply) = Message::new(
                header.id.clone(),
                MessageType::UploadChunkResponse,
                Some(&resp),
            ) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    fn on_complete_upload(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::CompleteUploadRequestFull = match msg.parse_payload() {
                Ok(Some(r)) => r,
                _ => {
                    let _ = sender.send_error(&msg, 400, "invalid payload");
                    return;
                }
            };

            let mut uploads = self.state.uploads.lock().await;
            let session = match uploads.remove(&req.upload_id) {
                Some(s) => s,
                None => {
                    let _ = sender.send_error(&msg, 404, "upload not found");
                    return;
                }
            };
            drop(uploads);

            let config = self.state.config.lock().await;
            let base_path = expand_path(&config.install_path);
            drop(config);
            let game_path = PathBuf::from(&base_path).join(&session.game_name);

            tracing::info!(
                "Upload completed: {} -> {}",
                req.upload_id,
                game_path.display()
            );

            self.emit_operation(
                &sender,
                "install",
                "complete",
                &session.game_name,
                100.0,
                "Instalación completada",
            );

            // Make executable on Linux
            #[cfg(target_os = "linux")]
            if !session.executable.is_empty() {
                let exe_path = game_path.join(&session.executable);
                if let Err(e) = std::fs::set_permissions(
                    &exe_path,
                    std::os::unix::fs::PermissionsExt::from_mode(0o755),
                ) {
                    tracing::warn!("failed to make executable: {e}");
                }
            }

            let mut resp = messages::CompleteUploadResponseFull {
                success: true,
                path: game_path.to_string_lossy().into(),
                app_id: 0,
            };

            // Create shortcut if requested
            #[allow(clippy::collapsible_if)]
            if req.create_shortcut {
                if let Some(shortcut_cfg) = req.shortcut {
                    let exe_name = std::path::Path::new(&shortcut_cfg.exe)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&session.executable);
                    let full_exe = game_path.join(exe_name).to_string_lossy().into_owned();
                    let start_dir = game_path.to_string_lossy().into_owned();

                    // Ensure CEF debug file exists.
                    let ctrl = capydeploy_steam::Controller::new();
                    if let Err(e) = ctrl.ensure_cef_debug_file() {
                        tracing::warn!("failed to ensure CEF debug file: {e}");
                    }

                    // Create shortcut via CEF API (like Go agent's manager.Create).
                    let cef = capydeploy_steam::CefClient::new();
                    let cef_timeout = std::time::Duration::from_secs(15);

                    match tokio::time::timeout(
                        cef_timeout,
                        cef.add_shortcut(
                            &shortcut_cfg.name,
                            &full_exe,
                            &start_dir,
                            &shortcut_cfg.launch_options,
                        ),
                    )
                    .await
                    {
                        Ok(Ok(app_id)) => {
                            resp.app_id = app_id;
                            tracing::info!(
                                "Created shortcut '{}' with AppID {} (exe={})",
                                shortcut_cfg.name,
                                app_id,
                                full_exe,
                            );

                            // AddShortcut ignores the name parameter and uses the
                            // executable filename, so rename it afterwards.
                            if let Err(e) = cef.set_shortcut_name(app_id, &shortcut_cfg.name).await
                            {
                                tracing::warn!("failed to set shortcut name: {e}");
                            }

                            // On Linux, auto-set Proton for Windows executables.
                            #[cfg(target_os = "linux")]
                            if full_exe.to_lowercase().ends_with(".exe")
                                && let Err(e) =
                                    cef.specify_compat_tool(app_id, "proton_experimental").await
                            {
                                tracing::warn!("failed to set Proton compat tool: {e}");
                            }

                            // Track the shortcut in memory (VDF may not be flushed yet).
                            {
                                let mut tracked = self.state.tracked_shortcuts.lock().await;
                                tracked.push(TrackedShortcut {
                                    app_id,
                                    name: shortcut_cfg.name.clone(),
                                    exe: full_exe.clone(),
                                    start_dir: start_dir.clone(),
                                });
                            }

                            // Notify agent UI about shortcut change.
                            let _ = self.app_handle.emit("shortcuts:changed", &());
                        }
                        Ok(Err(e)) => {
                            tracing::error!("CEF AddShortcut failed: {e}");
                            // Fallback: use calculated app_id for artwork.
                            resp.app_id =
                                capydeploy_steam::generate_app_id(&full_exe, &shortcut_cfg.name);
                        }
                        Err(_) => {
                            tracing::error!("CEF AddShortcut timed out (15s)");
                            resp.app_id =
                                capydeploy_steam::generate_app_id(&full_exe, &shortcut_cfg.name);
                        }
                    }

                    // Apply pending artwork using the real app_id from CEF.
                    let mut pending = self.state.pending_artwork.lock().await;
                    let artwork_items: Vec<_> = pending.drain(..).collect();
                    drop(pending);

                    if !artwork_items.is_empty() {
                        self.apply_pending_artwork(resp.app_id, artwork_items);
                    }
                }
            }

            if let Ok(reply) = msg.reply(MessageType::OperationResult, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    fn on_cancel_upload(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::CancelUploadRequest = match msg.parse_payload() {
                Ok(Some(r)) => r,
                _ => {
                    let _ = sender.send_error(&msg, 400, "invalid payload");
                    return;
                }
            };

            let mut uploads = self.state.uploads.lock().await;
            if let Some(session) = uploads.remove(&req.upload_id) {
                let config = self.state.config.lock().await;
                let base_path = expand_path(&config.install_path);
                drop(config);
                let game_path = PathBuf::from(&base_path).join(&session.game_name);

                // Clean up partial files
                if let Err(e) = std::fs::remove_dir_all(&game_path) {
                    tracing::warn!(
                        "failed to clean up partial upload at {}: {e}",
                        game_path.display()
                    );
                }

                tracing::info!(
                    "Upload cancelled: {} (cleaned {})",
                    req.upload_id,
                    game_path.display()
                );
            }
            drop(uploads);

            // Clean up pending artwork
            self.state.pending_artwork.lock().await.clear();

            let resp = messages::OperationResult {
                success: true,
                message: "cancelled".into(),
            };
            if let Ok(reply) = msg.reply(MessageType::OperationResult, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    // -----------------------------------------------------------------------
    // Binary artwork
    // -----------------------------------------------------------------------

    fn on_binary_artwork(
        &self,
        sender: Sender,
        header: BinaryArtworkHeader,
        data: Vec<u8>,
    ) -> HandlerFuture<'_> {
        Box::pin(async move {
            tracing::info!(
                "Received artwork image: appID={}, type={}, contentType={}, size={}",
                header.app_id,
                header.artwork_type,
                header.content_type,
                data.len()
            );

            if header.app_id == 0 {
                // Store for later — applied during complete_upload with real AppID
                self.state
                    .pending_artwork
                    .lock()
                    .await
                    .push(PendingArtwork {
                        artwork_type: header.artwork_type.clone(),
                        content_type: header.content_type.clone(),
                        data,
                    });
                tracing::info!("Stored pending artwork: type={}", header.artwork_type);

                let resp = messages::ArtworkImageResponse {
                    success: true,
                    artwork_type: header.artwork_type,
                    error: String::new(),
                };
                if let Ok(reply) = Message::new(
                    header.id.clone(),
                    MessageType::ArtworkImageResponse,
                    Some(&resp),
                ) {
                    let _ = sender.send_msg(reply);
                }
                return;
            }

            // Apply artwork immediately for known AppID
            let art_type = match parse_artwork_type(&header.artwork_type) {
                Some(t) => t,
                None => {
                    let resp = messages::ArtworkImageResponse {
                        success: false,
                        artwork_type: header.artwork_type,
                        error: "unknown artwork type".into(),
                    };
                    if let Ok(reply) = Message::new(
                        header.id.clone(),
                        MessageType::ArtworkImageResponse,
                        Some(&resp),
                    ) {
                        let _ = sender.send_msg(reply);
                    }
                    return;
                }
            };

            let ext = ext_from_content_type(&header.content_type);

            let result = (|| -> Result<(), String> {
                let users = capydeploy_steam::get_users().map_err(|e| e.to_string())?;
                let user = users
                    .first()
                    .ok_or_else(|| "no Steam users found".to_string())?;
                let sm = capydeploy_steam::ShortcutManager::new().map_err(|e| e.to_string())?;
                sm.save_artwork(&user.id, header.app_id, art_type, &data, ext)
                    .map_err(|e| e.to_string())
            })();

            match result {
                Ok(()) => {
                    let resp = messages::ArtworkImageResponse {
                        success: true,
                        artwork_type: header.artwork_type,
                        error: String::new(),
                    };
                    if let Ok(reply) = Message::new(
                        header.id.clone(),
                        MessageType::ArtworkImageResponse,
                        Some(&resp),
                    ) {
                        let _ = sender.send_msg(reply);
                    }
                }
                Err(e) => {
                    tracing::error!("failed to apply artwork image: {e}");
                    let resp = messages::ArtworkImageResponse {
                        success: false,
                        artwork_type: header.artwork_type,
                        error: e,
                    };
                    if let Ok(reply) = Message::new(
                        header.id.clone(),
                        MessageType::ArtworkImageResponse,
                        Some(&resp),
                    ) {
                        let _ = sender.send_msg(reply);
                    }
                }
            }
        })
    }

    // -----------------------------------------------------------------------
    // Console log + telemetry
    // -----------------------------------------------------------------------

    fn on_set_console_log_filter(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::SetConsoleLogFilterRequest = match msg.parse_payload() {
                Ok(Some(r)) => r,
                _ => {
                    let _ = sender.send_error(&msg, 400, "invalid payload");
                    return;
                }
            };

            // Apply filter to the collector
            self.state
                .console_log_collector
                .set_level_mask(req.level_mask);

            tracing::info!("Console log filter updated: mask=0x{:02x}", req.level_mask);

            let resp = messages::SetConsoleLogFilterResponse {
                level_mask: req.level_mask,
            };
            if let Ok(reply) = msg.reply(MessageType::SetConsoleLogFilter, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    fn on_set_console_log_enabled(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::SetConsoleLogEnabledRequest = match msg.parse_payload() {
                Ok(Some(r)) => r,
                _ => {
                    let _ = sender.send_error(&msg, 400, "invalid payload");
                    return;
                }
            };

            self.state
                .console_log_enabled
                .store(req.enabled, Ordering::Relaxed);

            // Start/stop the collector
            if req.enabled {
                self.state.console_log_collector.start().await;
            } else {
                self.state.console_log_collector.stop().await;
            }

            // Persist to config
            let mut config = self.state.config.lock().await;
            config.console_log_enabled = req.enabled;
            let _ = config.save();
            drop(config);

            tracing::info!("Console log enabled (remote): {}", req.enabled);
            self.emit_status_changed().await;

            let resp = messages::SetConsoleLogEnabledResponse {
                enabled: req.enabled,
            };
            if let Ok(reply) = msg.reply(MessageType::SetConsoleLogEnabled, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    fn on_set_game_log_wrapper(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::SetGameLogWrapperRequest = match msg.parse_payload() {
                Ok(Some(r)) => r,
                _ => {
                    let _ = sender.send_error(&msg, 400, "invalid payload");
                    return;
                }
            };

            tracing::info!(
                "Game log wrapper: appID={}, enabled={}",
                req.app_id,
                req.enabled
            );

            // TODO: wire to game-log crate
            let resp = messages::OperationResult {
                success: true,
                message: String::new(),
            };
            if let Ok(reply) = msg.reply(MessageType::OperationResult, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        })
    }

    // -----------------------------------------------------------------------
    // Disconnect
    // -----------------------------------------------------------------------

    fn on_hub_disconnected(&self) -> HandlerFuture<'_> {
        Box::pin(async move {
            tracing::info!("Hub disconnected");

            // Clear WS sender
            *self.state.hub_sender.lock().unwrap() = None;

            // Stop collectors
            self.state.telemetry_collector.stop().await;
            self.state.console_log_collector.stop().await;

            *self.state.connected_hub.lock().await = None;
            self.emit_status_changed().await;
        })
    }
}

// ---------------------------------------------------------------------------
// Helper methods
// ---------------------------------------------------------------------------

impl TauriAgentHandler {
    async fn accept_hub(
        &self,
        sender: &Sender,
        msg: &Message,
        req: &messages::HubConnectedRequest,
    ) {
        // Store WS sender for telemetry/console-log forwarding
        tracing::debug!(
            sender_connected = sender.is_connected(),
            "accept_hub: storing hub_sender"
        );
        *self.state.hub_sender.lock().unwrap() = Some(sender.clone());

        // Update connected hub state
        *self.state.connected_hub.lock().await = Some(ConnectedHubInfo {
            id: req.hub_id.clone(),
            name: req.name.clone(),
            ip: String::new(),
        });

        // Build agent status response
        let config = self.state.config.lock().await;
        let resp = messages::AgentStatusResponse {
            name: config.name.clone(),
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            accept_connections: self.state.accept_connections.load(Ordering::Relaxed),
            telemetry_enabled: config.telemetry_enabled,
            telemetry_interval: config.telemetry_interval,
            console_log_enabled: config.console_log_enabled,
        };

        // Start collectors based on config
        let telemetry_enabled = config.telemetry_enabled;
        let telemetry_interval = config.telemetry_interval;
        let console_log_enabled = config.console_log_enabled;
        drop(config);

        if let Ok(reply) = msg.reply(MessageType::AgentStatus, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }

        // Start telemetry if enabled and notify Hub
        if telemetry_enabled {
            self.state
                .telemetry_collector
                .start(telemetry_interval as u32)
                .await;
        }
        crate::commands::notify_telemetry_status(
            &self.state,
            telemetry_enabled,
            telemetry_interval,
        );

        // Start console log if enabled and notify Hub
        if console_log_enabled {
            self.state.console_log_collector.start().await;
        }
        crate::commands::notify_console_log_status(&self.state, console_log_enabled);

        self.emit_status_changed().await;
    }

    async fn emit_status_changed(&self) {
        let config = self.state.config.lock().await;
        let hub = self.state.connected_hub.lock().await;
        let port = *self.state.server_port.lock().await;

        let status = crate::types::AgentStatusDto {
            running: true,
            name: config.name.clone(),
            platform: std::env::consts::OS.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            port,
            ips: local_ips(),
            accept_connections: self.state.accept_connections.load(Ordering::Relaxed),
            connected_hub: hub.as_ref().map(|h| ConnectedHubDto {
                id: h.id.clone(),
                name: h.name.clone(),
                ip: h.ip.clone(),
            }),
            telemetry_enabled: config.telemetry_enabled,
            telemetry_interval: config.telemetry_interval,
            console_log_enabled: config.console_log_enabled,
        };
        let _ = self.app_handle.emit("status:changed", &status);
    }

    fn emit_operation(
        &self,
        sender: &Sender,
        event_type: &str,
        status: &str,
        game_name: &str,
        progress: f64,
        message: &str,
    ) {
        let evt = messages::OperationEvent {
            event_type: event_type.into(),
            status: status.into(),
            game_name: game_name.into(),
            progress,
            message: message.into(),
        };
        // Send to Hub via WS
        self.send_event(sender, MessageType::OperationEvent, &evt);
        // Emit to local UI
        let dto = crate::types::OperationEventDto {
            event_type: event_type.into(),
            status: status.into(),
            game_name: game_name.into(),
            progress,
            message: message.into(),
        };
        let _ = self.app_handle.emit("operation", &dto);
    }

    fn send_event<T: serde::Serialize>(&self, sender: &Sender, msg_type: MessageType, payload: &T) {
        let id = uuid::Uuid::new_v4().to_string();
        if let Ok(msg) = Message::new(id, msg_type, Some(payload)) {
            let _ = sender.send_msg(msg);
        }
    }

    /// Applies buffered pending artwork for a given app_id.
    /// Applies pending artwork via CEF API (instant), with filesystem fallback.
    ///
    /// Spawns a background task so the handler doesn't block waiting for
    /// multiple sequential CEF calls.
    fn apply_pending_artwork(&self, app_id: u32, artwork_items: Vec<PendingArtwork>) {
        use base64::Engine;

        tokio::spawn(async move {
            let cef = capydeploy_steam::CefClient::new();

            for pa in &artwork_items {
                let asset_type = match capydeploy_steam::artwork_type_to_cef_asset(&pa.artwork_type)
                {
                    Some(t) => t,
                    None => {
                        tracing::warn!("unknown artwork type: {}", pa.artwork_type);
                        continue;
                    }
                };

                let b64 = base64::engine::general_purpose::STANDARD.encode(&pa.data);

                match cef.set_custom_artwork(app_id, &b64, asset_type).await {
                    Ok(()) => {
                        tracing::info!(
                            "Applied artwork via CEF: appID={}, type={}",
                            app_id,
                            pa.artwork_type
                        );
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "CEF artwork failed for {} (appID {}), falling back to filesystem: {e}",
                            pa.artwork_type,
                            app_id
                        );
                    }
                }

                // Filesystem fallback (requires Steam restart to show).
                let sm = match capydeploy_steam::ShortcutManager::new() {
                    Ok(sm) => sm,
                    Err(e) => {
                        tracing::warn!("failed to init ShortcutManager for artwork fallback: {e}");
                        continue;
                    }
                };
                let users = capydeploy_steam::get_users().unwrap_or_default();
                if users.is_empty() {
                    tracing::warn!("no Steam users found for artwork fallback");
                    continue;
                }
                let art_type = match parse_artwork_type(&pa.artwork_type) {
                    Some(t) => t,
                    None => continue,
                };
                let ext = ext_from_content_type(&pa.content_type);
                if let Err(e) = sm.save_artwork(&users[0].id, app_id, art_type, &pa.data, ext) {
                    tracing::warn!(
                        "filesystem artwork fallback failed for {}: {e}",
                        pa.artwork_type
                    );
                }
            }
        });
    }
}

/// Generates a stable agent ID from the name.
fn generate_agent_id(name: &str) -> String {
    use std::fmt::Write;
    let platform = std::env::consts::OS;
    let data = format!("{name}-{platform}-agent");
    let digest = <sha2::Sha256 as sha2::Digest>::digest(data.as_bytes());
    let mut hex = String::with_capacity(8);
    for byte in &digest[..4] {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Expands ~ in paths.
fn expand_path(path: &str) -> String {
    if path.starts_with("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{}{}", home, &path[1..]);
    }
    path.to_string()
}

/// Safely deletes a game directory.
///
/// Safety checks (matching Go agent):
/// - Must be an absolute path
/// - Must be within the user's home directory
/// - Must be at least 2 levels deep from home (e.g. ~/Games/MyGame, not ~/Games)
fn delete_game_directory(path: &str) -> Result<(), String> {
    let abs_path = std::path::Path::new(path);
    if !abs_path.is_absolute() {
        return Err(format!("refusing to delete relative path: {path}"));
    }

    let home = std::env::var("HOME").map_err(|_| "cannot determine home directory".to_string())?;
    let home_path = std::path::Path::new(&home);

    if !abs_path.starts_with(home_path) {
        return Err(format!(
            "refusing to delete path outside home directory: {}",
            abs_path.display()
        ));
    }

    let rel_path = abs_path
        .strip_prefix(home_path)
        .map_err(|_| "cannot determine relative path".to_string())?;

    // Must be at least 2 levels deep (e.g., Games/MyGame, not just Games).
    let depth = rel_path.components().count();
    if depth < 2 {
        return Err(format!(
            "refusing to delete top-level directory: {}",
            abs_path.display()
        ));
    }

    match std::fs::metadata(abs_path) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(format!("path is not a directory: {}", abs_path.display()));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(format!("cannot stat path: {e}"));
        }
    }

    std::fs::remove_dir_all(abs_path).map_err(|e| format!("failed to remove directory: {e}"))
}
