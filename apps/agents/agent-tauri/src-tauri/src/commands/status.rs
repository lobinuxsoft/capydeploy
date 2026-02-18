use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::State;

use crate::state::AgentState;
use crate::types::{AgentStatusDto, ConnectedHubDto, VersionInfoDto};

#[tauri::command]
pub async fn get_status(state: State<'_, Arc<AgentState>>) -> Result<AgentStatusDto, String> {
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

    Ok(AgentStatusDto {
        running: true,
        name: config.name.clone(),
        platform: std::env::consts::OS.into(),
        version: env!("CAPYDEPLOY_VERSION").into(),
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
    })
}

#[tauri::command]
pub async fn get_version() -> Result<VersionInfoDto, String> {
    Ok(VersionInfoDto {
        version: env!("CAPYDEPLOY_VERSION").into(),
        commit: String::new(),
        build_date: String::new(),
    })
}
