use capydeploy_agent_server::{BinaryArtworkHeader, Sender};
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::handler::TauriAgentHandler;
use crate::helpers::{ext_from_content_type, parse_artwork_type};
use crate::state::PendingArtwork;

impl TauriAgentHandler {
    pub(crate) async fn handle_binary_artwork(
        &self,
        sender: Sender,
        header: BinaryArtworkHeader,
        data: Vec<u8>,
    ) {
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
    }

    /// Applies buffered pending artwork for a given app_id.
    /// Applies pending artwork via CEF API (instant), with filesystem fallback.
    ///
    /// Spawns a background task so the handler doesn't block waiting for
    /// multiple sequential CEF calls.
    pub(crate) fn apply_pending_artwork(&self, app_id: u32, artwork_items: Vec<PendingArtwork>) {
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
