use std::path::PathBuf;

use tauri::Emitter;

use capydeploy_agent_server::{BinaryChunkHeader, Sender};
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::handler::TauriAgentHandler;
use crate::helpers::expand_path;
use crate::state::{TrackedShortcut, UploadSession};

impl TauriAgentHandler {
    pub(crate) async fn handle_init_upload(&self, sender: Sender, msg: Message) {
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

        if let Err(e) = capydeploy_transfer::validate_upload_path(&req.config.game_name) {
            let _ = sender.send_error(&msg, 400, &format!("invalid game name: {e}"));
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
    }

    pub(crate) async fn handle_upload_chunk(&self, sender: Sender, msg: Message) {
        // Handled by binary path — JSON chunks are a fallback
        let _ = sender.send_error(&msg, 501, "use binary chunk protocol");
    }

    pub(crate) async fn handle_binary_chunk(
        &self,
        sender: Sender,
        header: BinaryChunkHeader,
        data: Vec<u8>,
    ) {
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
    }

    pub(crate) async fn handle_complete_upload(&self, sender: Sender, msg: Message) {
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
                        if let Err(e) = cef.set_shortcut_name(app_id, &shortcut_cfg.name).await {
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
    }

    pub(crate) async fn handle_cancel_upload(&self, sender: Sender, msg: Message) {
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
    }
}
