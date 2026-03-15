//! Filesystem browsing Tauri commands.
//!
//! All file data transfers use TCP data channels (never WebSocket).
//! WS is only used for signaling (metadata, TCP port/token exchange).

pub mod commands;
pub mod download;
pub mod upload;

// FsTransferState is defined here and used by submodules via super::.

use std::net::SocketAddr;

use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::messages::{FsListRequest, FsListResponse, OperationResult};

use crate::state::HubState;
use crate::types::UploadProgressDto;

// ---------------------------------------------------------------------------
// Transfer state (shared between commands and cancel)
// ---------------------------------------------------------------------------

/// Active file browser transfer state — stored in Tauri managed state.
pub struct FsTransferState {
    pub cancel: tokio::sync::Mutex<Option<CancellationToken>>,
}

impl FsTransferState {
    pub fn new() -> Self {
        Self {
            cancel: tokio::sync::Mutex::new(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

pub(super) fn emit_progress(
    app: &AppHandle,
    status: &str,
    progress: f64,
    done: bool,
    error: Option<&str>,
) {
    let dto = UploadProgressDto {
        progress,
        status: status.to_string(),
        error: error.map(|s| s.to_string()),
        done,
    };
    let _ = app.emit("filebrowser:progress", &dto);
}

pub(super) fn format_bytes(bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let i = (bytes as f64).log(1024.0).floor() as usize;
    let i = i.min(UNITS.len() - 1);
    let val = bytes as f64 / 1024f64.powi(i as i32);
    format!("{val:.1} {}", UNITS[i])
}

pub(super) async fn send_and_check<T: serde::Serialize>(
    state: &State<'_, HubState>,
    msg_type: MessageType,
    payload: Option<&T>,
) -> Result<(), String> {
    let mgr = state.connection_mgr.clone();
    let resp = mgr
        .send_request(msg_type, payload)
        .await
        .map_err(|e| e.to_string())?;
    let result = resp
        .parse_payload::<OperationResult>()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "empty response".to_string())?;
    if result.success {
        Ok(())
    } else {
        Err(result.message)
    }
}

pub(super) async fn agent_tcp_addr(
    state: &State<'_, HubState>,
    tcp_port: u16,
) -> Result<SocketAddr, String> {
    let connected = state
        .connection_mgr
        .get_connected()
        .await
        .ok_or_else(|| "not connected".to_string())?;
    let agent_ip = connected
        .agent
        .ips
        .first()
        .ok_or_else(|| "agent has no known IP address".to_string())?;
    Ok(SocketAddr::new(*agent_ip, tcp_port))
}

pub(super) async fn count_remote_bytes(
    state: &State<'_, HubState>,
    remote_dir: &str,
) -> Result<i64, String> {
    let mgr = state.connection_mgr.clone();
    let req = FsListRequest {
        path: remote_dir.to_string(),
        show_hidden: true,
    };
    let resp = mgr
        .send_request(MessageType::FsList, Some(&req))
        .await
        .map_err(|e| e.to_string())?;
    let listing = resp
        .parse_payload::<FsListResponse>()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "empty response".to_string())?;
    let mut total = 0i64;
    for entry in &listing.entries {
        if entry.is_dir {
            total += Box::pin(count_remote_bytes(state, &entry.path)).await?;
        } else {
            total += entry.size;
        }
    }
    Ok(total)
}

pub(super) async fn get_remote_file_size(
    state: &State<'_, HubState>,
    remote_path: &str,
) -> Result<i64, String> {
    let mgr = state.connection_mgr.clone();
    let parent = std::path::Path::new(remote_path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = std::path::Path::new(remote_path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let req = FsListRequest {
        path: parent,
        show_hidden: true,
    };
    let resp = mgr
        .send_request(MessageType::FsList, Some(&req))
        .await
        .map_err(|e| e.to_string())?;
    let listing = resp
        .parse_payload::<FsListResponse>()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "empty response".to_string())?;

    Ok(listing
        .entries
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.size)
        .unwrap_or(0))
}
