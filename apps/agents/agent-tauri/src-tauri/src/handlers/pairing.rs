use std::sync::atomic::Ordering;

use tauri::Emitter;

use capydeploy_agent_server::Sender;
use capydeploy_protocol::constants::{
    self, MessageType, PROTOCOL_VERSION, check_protocol_compatibility,
};
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::config::AuthorizedHub;
use crate::handler::TauriAgentHandler;
use crate::state::ConnectedHubInfo;

impl TauriAgentHandler {
    pub(crate) async fn handle_hub_connected(&self, sender: Sender, msg: Message) {
        let req: messages::HubConnectedRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ = sender.send_error(&msg, 400, "invalid payload");
                return;
            }
        };

        tracing::info!(
            "Hub connected: {} v{} (ID: {}, proto: v{})",
            req.name,
            req.version,
            req.hub_id,
            req.protocol_version
        );

        // Reject incompatible protocol versions before any auth.
        if let constants::ProtocolCompatibility::Incompatible { reason, .. } =
            check_protocol_compatibility(req.protocol_version)
        {
            tracing::warn!("Rejecting hub {}: {reason}", req.hub_id);
            let _ = sender.send_error(&msg, 406, &reason);
            return;
        }

        // If Hub provided a token, validate it
        if !req.token.is_empty() && !req.hub_id.is_empty() {
            let config = self.state.config.lock().await;
            if crate::auth::AuthManager::validate_token(
                &config.authorized_hubs,
                &req.hub_id,
                &req.token,
            ) {
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
    }

    pub(crate) async fn handle_pair_confirm(&self, sender: Sender, msg: Message) {
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
    }

    pub(crate) async fn accept_hub(
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
            version: env!("CAPYDEPLOY_VERSION").into(),
            platform: std::env::consts::OS.into(),
            accept_connections: self.state.accept_connections.load(Ordering::Relaxed),
            telemetry_enabled: config.telemetry_enabled,
            telemetry_interval: config.telemetry_interval,
            console_log_enabled: config.console_log_enabled,
            protocol_version: PROTOCOL_VERSION,
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
}
