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

        #[cfg(target_os = "linux")]
        {
            match self
                .state
                .game_log_wrapper
                .set_wrapper(req.app_id, req.enabled)
                .await
            {
                Ok(resp) => {
                    // Start/stop log watcher.
                    if req.enabled {
                        let log_dir = capydeploy_game_log::log_dir();
                        self.state
                            .game_log_tailer
                            .start_watch(req.app_id, log_dir)
                            .await;
                    } else {
                        self.state.game_log_tailer.stop_tail(req.app_id).await;
                    }

                    // Reply with wrapper response.
                    if let Ok(reply) = msg.reply(MessageType::SetGameLogWrapper, Some(&resp)) {
                        let _ = sender.send_msg(reply);
                    }

                    // Broadcast updated wrapper status.
                    self.send_game_log_wrapper_status(&sender).await;
                }
                Err(e) => {
                    tracing::error!("game log wrapper failed: {e}");
                    let _ = sender.send_error(&msg, 500, &e);
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = sender.send_error(&msg, 501, "game log wrapper not supported on this platform");
        }
    }

    #[allow(unused_variables)]
    async fn send_game_log_wrapper_status(&self, sender: &Sender) {
        #[cfg(target_os = "linux")]
        {
            let status = self.state.game_log_wrapper.status().await;
            let id = uuid::Uuid::new_v4().to_string();
            if let Ok(msg) = Message::new(id, MessageType::GameLogWrapperStatus, Some(&status)) {
                let _ = sender.send_msg(msg);
            }
        }
    }
}
