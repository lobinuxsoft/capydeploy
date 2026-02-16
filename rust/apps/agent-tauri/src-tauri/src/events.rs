//! Background tasks: WS server, mDNS discovery, telemetry/console-log forwarding.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use capydeploy_agent_server::{AgentServer, ServerConfig};
use capydeploy_discovery::{Server as DiscoveryServer, ServiceInfo};

use crate::handler::TauriAgentHandler;
use crate::state::AgentState;
use crate::types::AgentStatusDto;

/// Starts the WS server and mDNS discovery.
pub async fn start_server(handle: AppHandle, state: Arc<AgentState>) {
    let handler = TauriAgentHandler {
        app_handle: handle.clone(),
        state: state.clone(),
    };

    let config = state.config.lock().await;
    let agent_name = config.name.clone();
    drop(config);

    let server_config = ServerConfig {
        port: 0, // OS-assigned
    };

    // Share the same AtomicBool: when the Tauri command toggles it,
    // the server sees it immediately (no extra sync needed).
    let server = AgentServer::new(server_config, handler, state.accept_connections.clone());
    let server_clone = Arc::clone(&server);

    // Spawn server
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server_clone.run().await {
            tracing::error!("agent server error: {e}");
        }
    });

    // Wait for the server to bind
    tokio::time::sleep(Duration::from_millis(100)).await;

    let port = server.port().await;
    *state.server_port.lock().await = port;
    tracing::info!("Agent server listening on port {port}");

    // Emit initial status
    emit_status(&handle, &state).await;

    // Start mDNS discovery advertisement
    let mut discovery = start_discovery(&agent_name, port);

    // Wait for server to complete
    let _ = server_handle.await;

    // Shutdown discovery
    if let Some(disc) = discovery.as_mut() {
        let _ = disc.stop();
    }
}

fn start_discovery(name: &str, port: u16) -> Option<DiscoveryServer> {
    // Generate a stable agent ID from the name
    let id = {
        use std::fmt::Write;
        let platform = std::env::consts::OS;
        let data = format!("{name}-{platform}-agent");
        let digest = <sha2::Sha256 as sha2::Digest>::digest(data.as_bytes());
        let mut hex = String::with_capacity(8);
        for byte in &digest[..4] {
            let _ = write!(hex, "{byte:02x}");
        }
        hex
    };

    let info = ServiceInfo {
        id,
        name: name.to_string(),
        platform: std::env::consts::OS.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        port,
        ips: vec![],
    };

    let mut server = DiscoveryServer::new(info);
    match server.start() {
        Ok(()) => {
            tracing::info!("mDNS discovery advertising on port {port}");
            Some(server)
        }
        Err(e) => {
            tracing::warn!("failed to start mDNS discovery: {e}");
            None
        }
    }
}

async fn emit_status(handle: &AppHandle, state: &AgentState) {
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

    let status = AgentStatusDto {
        running: true,
        name: config.name.clone(),
        platform: std::env::consts::OS.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        port,
        ips,
        accept_connections: state.accept_connections.load(Ordering::Relaxed),
        connected_hub: hub.as_ref().map(|h| crate::types::ConnectedHubDto {
            id: h.id.clone(),
            name: h.name.clone(),
            ip: h.ip.clone(),
        }),
        telemetry_enabled: config.telemetry_enabled,
        telemetry_interval: config.telemetry_interval,
        console_log_enabled: config.console_log_enabled,
    };

    let _ = handle.emit("server:started", &status);
    let _ = handle.emit("status:changed", &status);
}
