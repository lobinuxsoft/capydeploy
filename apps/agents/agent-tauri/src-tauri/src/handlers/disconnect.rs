use crate::handler::TauriAgentHandler;

impl TauriAgentHandler {
    pub(crate) async fn handle_hub_disconnected(&self) {
        tracing::info!("Hub disconnected");

        // Clear WS sender
        *self.state.hub_sender.lock().unwrap() = None;

        // Stop collectors
        self.state.telemetry_collector.stop().await;
        self.state.console_log_collector.stop().await;

        *self.state.connected_hub.lock().await = None;
        self.emit_status_changed().await;
    }
}
