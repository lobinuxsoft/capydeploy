//! Handler implementation for the agent WS server.
//!
//! `TauriAgentHandler` implements the `agent_server::Handler` trait,
//! dispatching all Hub→Agent messages to domain-specific handler modules.
//!
//! Domain logic lives in `handlers/` sub-modules; this file contains only
//! the struct definition, trait dispatch, and cross-cutting emit helpers.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter};

use capydeploy_agent_server::{
    BinaryArtworkHeader, BinaryChunkHeader, Handler, HandlerFuture, Sender,
};
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;

use crate::helpers::local_ips;
use crate::state::AgentState;
use crate::types::{AgentStatusDto, ConnectedHubDto, OperationEventDto};

pub struct TauriAgentHandler {
    pub app_handle: AppHandle,
    pub state: Arc<AgentState>,
}

// ---------------------------------------------------------------------------
// Handler trait — thin dispatch to handlers/*
// ---------------------------------------------------------------------------

impl Handler for TauriAgentHandler {
    fn on_hub_connected(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_hub_connected(sender, msg))
    }

    fn on_pair_confirm(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_pair_confirm(sender, msg))
    }

    fn on_get_info(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_get_info(sender, msg))
    }

    fn on_get_config(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_get_config(sender, msg))
    }

    fn on_get_steam_users(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_get_steam_users(sender, msg))
    }

    fn on_list_shortcuts(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_list_shortcuts(sender, msg))
    }

    fn on_create_shortcut(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_create_shortcut(sender, msg))
    }

    fn on_delete_shortcut(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_delete_shortcut(sender, msg))
    }

    fn on_delete_game(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_delete_game(sender, msg))
    }

    fn on_apply_artwork(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_apply_artwork(sender, msg))
    }

    fn on_restart_steam(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_restart_steam(sender, msg))
    }

    fn on_init_upload(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_init_upload(sender, msg))
    }

    fn on_upload_chunk(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_upload_chunk(sender, msg))
    }

    fn on_binary_chunk(
        &self,
        sender: Sender,
        header: BinaryChunkHeader,
        data: Vec<u8>,
    ) -> HandlerFuture<'_> {
        Box::pin(self.handle_binary_chunk(sender, header, data))
    }

    fn on_complete_upload(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_complete_upload(sender, msg))
    }

    fn on_cancel_upload(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_cancel_upload(sender, msg))
    }

    fn on_binary_artwork(
        &self,
        sender: Sender,
        header: BinaryArtworkHeader,
        data: Vec<u8>,
    ) -> HandlerFuture<'_> {
        Box::pin(self.handle_binary_artwork(sender, header, data))
    }

    fn on_set_console_log_filter(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_set_console_log_filter(sender, msg))
    }

    fn on_set_console_log_enabled(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_set_console_log_enabled(sender, msg))
    }

    fn on_set_game_log_wrapper(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(self.handle_set_game_log_wrapper(sender, msg))
    }

    fn on_hub_disconnected(&self) -> HandlerFuture<'_> {
        Box::pin(self.handle_hub_disconnected())
    }
}

// ---------------------------------------------------------------------------
// Cross-cutting emit helpers (used by multiple handler domains)
// ---------------------------------------------------------------------------

impl TauriAgentHandler {
    pub(crate) async fn emit_status_changed(&self) {
        let config = self.state.config.lock().await;
        let hub = self.state.connected_hub.lock().await;
        let port = *self.state.server_port.lock().await;

        let status = AgentStatusDto {
            running: true,
            name: config.name.clone(),
            platform: std::env::consts::OS.into(),
            version: env!("CAPYDEPLOY_VERSION").into(),
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

    pub(crate) fn emit_operation(
        &self,
        sender: &Sender,
        event_type: &str,
        status: &str,
        game_name: &str,
        progress: f64,
        message: &str,
    ) {
        let evt = capydeploy_protocol::messages::OperationEvent {
            event_type: event_type.into(),
            status: status.into(),
            game_name: game_name.into(),
            progress,
            message: message.into(),
        };
        // Send to Hub via WS
        self.send_event(sender, MessageType::OperationEvent, &evt);
        // Emit to local UI
        let dto = OperationEventDto {
            event_type: event_type.into(),
            status: status.into(),
            game_name: game_name.into(),
            progress,
            message: message.into(),
        };
        let _ = self.app_handle.emit("operation", &dto);
    }

    pub(crate) fn send_event<T: serde::Serialize>(
        &self,
        sender: &Sender,
        msg_type: MessageType,
        payload: &T,
    ) {
        let id = uuid::Uuid::new_v4().to_string();
        if let Ok(msg) = Message::new(id, msg_type, Some(payload)) {
            let _ = sender.send_msg(msg);
        }
    }
}
