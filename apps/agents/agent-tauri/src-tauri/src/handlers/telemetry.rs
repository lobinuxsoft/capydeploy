use std::sync::atomic::Ordering;

use capydeploy_agent_server::Sender;
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::handler::TauriAgentHandler;

impl TauriAgentHandler {
    pub(crate) async fn handle_set_console_log_filter(&self, sender: Sender, msg: Message) {
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
    }

    pub(crate) async fn handle_set_console_log_enabled(&self, sender: Sender, msg: Message) {
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
    }

    pub(crate) async fn handle_set_game_log_wrapper(&self, sender: Sender, msg: Message) {
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
    }
}
