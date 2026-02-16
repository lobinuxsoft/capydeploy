pub mod auth;
pub mod connections;
pub mod console_log;
pub mod files;
pub mod settings;
pub mod status;
pub mod steam;
pub mod telemetry;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter};

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;

use crate::state::AgentState;
use crate::types::{AgentStatusDto, ConnectedHubDto};

/// Emits a full `AgentStatusDto` on the `status:changed` event.
/// Must be used instead of `emit("status:changed", &())`.
pub async fn emit_status(app: &AppHandle, state: &Arc<AgentState>) {
    let config = state.config.lock().await;
    let hub = state.connected_hub.lock().await;
    let port = *state.server_port.lock().await;

    let ips: Vec<String> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| {
            if iface.is_loopback() {
                return None;
            }
            match iface.addr.ip() {
                std::net::IpAddr::V4(ip) => {
                    if ip.octets()[0] == 169 && ip.octets()[1] == 254 {
                        return None;
                    }
                    Some(ip.to_string())
                }
                _ => None,
            }
        })
        .collect();

    let dto = AgentStatusDto {
        running: true,
        name: config.name.clone(),
        platform: std::env::consts::OS.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        port,
        ips,
        accept_connections: state.accept_connections.load(Ordering::Relaxed),
        connected_hub: hub.as_ref().map(|h| ConnectedHubDto {
            id: h.id.clone(),
            name: h.name.clone(),
            ip: h.ip.clone(),
        }),
        telemetry_enabled: config.telemetry_enabled,
        telemetry_interval: config.telemetry_interval,
        console_log_enabled: config.console_log_enabled,
    };

    let _ = app.emit("status:changed", &dto);
}

/// Sends a push event to the connected Hub over WebSocket.
fn send_hub_event<T: serde::Serialize>(state: &AgentState, msg_type: MessageType, payload: &T) {
    let sender = state.hub_sender.lock().unwrap();
    if let Some(ws) = sender.as_ref() {
        let id = uuid::Uuid::new_v4().to_string();
        match Message::new(id, msg_type.clone(), Some(payload)) {
            Ok(msg) => {
                if let Err(e) = ws.send_msg(msg) {
                    tracing::warn!("failed to send {msg_type:?} to hub: {e}");
                }
            }
            Err(e) => tracing::warn!("failed to build {msg_type:?} message: {e}"),
        }
    }
}

/// Notifies the Hub of the current telemetry status.
pub fn notify_telemetry_status(state: &AgentState, enabled: bool, interval: i32) {
    use capydeploy_protocol::telemetry::TelemetryStatusEvent;
    send_hub_event(
        state,
        MessageType::TelemetryStatus,
        &TelemetryStatusEvent { enabled, interval },
    );
}

/// Notifies the Hub of the current console log status.
pub fn notify_console_log_status(state: &AgentState, enabled: bool) {
    use capydeploy_protocol::console_log::ConsoleLogStatusEvent;
    send_hub_event(
        state,
        MessageType::ConsoleLogStatus,
        &ConsoleLogStatusEvent {
            enabled,
            level_mask: state.console_log_collector.get_level_mask(),
        },
    );
}
