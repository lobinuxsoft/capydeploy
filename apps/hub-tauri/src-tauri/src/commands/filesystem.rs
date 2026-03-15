//! Filesystem browsing Tauri commands.
//!
//! All file data transfers use TCP data channels (never WebSocket).
//! WS is only used for signaling (metadata, TCP port/token exchange).

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::messages::{
    FsDeleteRequest, FsDownloadReadyResponse, FsDownloadRequest, FsListRequest, FsListResponse,
    FsMkdirRequest, FsRenameRequest, FsUploadReadyResponse, FsUploadRequest, OperationResult,
};

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
// Progress helper
// ---------------------------------------------------------------------------

fn emit_progress(app: &AppHandle, status: &str, progress: f64, done: bool, error: Option<&str>) {
    let dto = UploadProgressDto {
        progress,
        status: status.to_string(),
        error: error.map(|s| s.to_string()),
        done,
    };
    let _ = app.emit("filebrowser:progress", &dto);
}

/// Format bytes as human-readable string.
fn format_bytes(bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let i = (bytes as f64).log(1024.0).floor() as usize;
    let i = i.min(UNITS.len() - 1);
    let val = bytes as f64 / 1024f64.powi(i as i32);
    format!("{val:.1} {}", UNITS[i])
}

// ---------------------------------------------------------------------------
// Read-only commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn fs_list(
    state: State<'_, HubState>,
    path: String,
    show_hidden: bool,
) -> Result<FsListResponse, String> {
    let mgr = state.connection_mgr.clone();
    let req = FsListRequest { path, show_hidden };
    let resp = mgr
        .send_request(MessageType::FsList, Some(&req))
        .await
        .map_err(|e| e.to_string())?;
    resp.parse_payload::<FsListResponse>()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "empty fs_list response".to_string())
}

#[tauri::command]
pub async fn fs_mkdir(state: State<'_, HubState>, path: String) -> Result<(), String> {
    send_and_check(&state, MessageType::FsMkdir, Some(&FsMkdirRequest { path })).await
}

#[tauri::command]
pub async fn fs_delete(state: State<'_, HubState>, path: String) -> Result<(), String> {
    send_and_check(
        &state,
        MessageType::FsDelete,
        Some(&FsDeleteRequest { path }),
    )
    .await
}

#[tauri::command]
pub async fn fs_rename(
    state: State<'_, HubState>,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    send_and_check(
        &state,
        MessageType::FsRename,
        Some(&FsRenameRequest { old_path, new_path }),
    )
    .await
}

// ---------------------------------------------------------------------------
// Cancel command
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn fs_cancel_transfer(fs_state: State<'_, FsTransferState>) -> Result<(), String> {
    let mut guard = fs_state.cancel.lock().await;
    if let Some(token) = guard.take() {
        token.cancel();
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Download commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn fs_download_path(
    app: AppHandle,
    state: State<'_, HubState>,
    fs_state: State<'_, FsTransferState>,
    path: String,
    is_dir: bool,
) -> Result<u32, String> {
    if is_dir {
        let local_dir = app.dialog().file().blocking_pick_folder();
        let local_dir = match local_dir {
            Some(p) => p.as_path().map(|p| p.to_path_buf()),
            None => return Ok(0),
        };
        let local_dir = local_dir.ok_or_else(|| "invalid folder".to_string())?;

        let dir_name = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "download".to_string());

        let dest = local_dir.join(&dir_name);
        std::fs::create_dir_all(&dest).map_err(|e| format!("failed to create directory: {e}"))?;

        emit_progress(&app, "Scanning directory...", 0.0, false, None);
        let total_bytes = count_remote_bytes(&state, &path).await.unwrap_or(0);

        let cancel = CancellationToken::new();
        *fs_state.cancel.lock().await = Some(cancel.clone());

        let transferred = Arc::new(AtomicI64::new(0));

        let result = download_directory_recursive(
            &app,
            &state,
            &path,
            &dest,
            &transferred,
            total_bytes,
            &cancel,
        )
        .await;

        *fs_state.cancel.lock().await = None;

        match &result {
            Ok(n) => emit_progress(&app, &format!("{n} file(s) downloaded"), 1.0, true, None),
            Err(e) => {
                emit_progress(&app, "Download failed", 0.0, true, Some(e));
                // Clean up partial directory on failure.
                let _ = std::fs::remove_dir_all(&dest);
            }
        }
        result
    } else {
        let filename = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "download".to_string());

        let save_path = app
            .dialog()
            .file()
            .set_file_name(&filename)
            .blocking_save_file();
        let save_path = match save_path {
            Some(p) => p.as_path().map(|p| p.to_path_buf()),
            None => return Ok(0),
        };
        let save_path = save_path.ok_or_else(|| "invalid save path".to_string())?;

        let cancel = CancellationToken::new();
        *fs_state.cancel.lock().await = Some(cancel.clone());

        emit_progress(
            &app,
            &format!("Downloading {filename}..."),
            0.0,
            false,
            None,
        );
        let result = download_single_file(&app, &state, &path, &save_path, 0, 0, &cancel).await;

        *fs_state.cancel.lock().await = None;

        match &result {
            Ok(()) => emit_progress(&app, &format!("{filename} saved"), 1.0, true, None),
            Err(e) => {
                emit_progress(&app, "Download failed", 0.0, true, Some(e));
                let _ = std::fs::remove_file(&save_path);
            }
        }
        result.map(|()| 1)
    }
}

/// Download multiple items to a single local folder.
#[tauri::command]
pub async fn fs_download_batch(
    app: AppHandle,
    state: State<'_, HubState>,
    fs_state: State<'_, FsTransferState>,
    paths: Vec<(String, bool)>,
) -> Result<u32, String> {
    let local_dir = app.dialog().file().blocking_pick_folder();
    let local_dir = match local_dir {
        Some(p) => p.as_path().map(|p| p.to_path_buf()),
        None => return Ok(0),
    };
    let local_dir = local_dir.ok_or_else(|| "invalid folder".to_string())?;

    emit_progress(&app, "Scanning...", 0.0, false, None);
    let mut total_bytes = 0i64;
    for (path, is_dir) in &paths {
        if *is_dir {
            total_bytes += count_remote_bytes(&state, path).await.unwrap_or(0);
        } else {
            total_bytes += get_remote_file_size(&state, path).await.unwrap_or(0);
        }
    }

    let cancel = CancellationToken::new();
    *fs_state.cancel.lock().await = Some(cancel.clone());
    let transferred = Arc::new(AtomicI64::new(0));
    let mut downloaded = 0u32;

    for (remote_path, is_dir) in &paths {
        if cancel.is_cancelled() {
            break;
        }
        if *is_dir {
            let dir_name = std::path::Path::new(remote_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "download".to_string());
            let dest = local_dir.join(&dir_name);
            std::fs::create_dir_all(&dest)
                .map_err(|e| format!("failed to create directory: {e}"))?;
            downloaded += download_directory_recursive(
                &app,
                &state,
                remote_path,
                &dest,
                &transferred,
                total_bytes,
                &cancel,
            )
            .await?;
        } else {
            let name = std::path::Path::new(remote_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "download".to_string());
            let local_file = local_dir.join(&name);
            download_single_file(
                &app,
                &state,
                remote_path,
                &local_file,
                transferred.load(Ordering::Relaxed),
                total_bytes,
                &cancel,
            )
            .await?;
            // Update transferred with file size.
            let size = get_remote_file_size(&state, remote_path).await.unwrap_or(0);
            transferred.fetch_add(size, Ordering::Relaxed);
            downloaded += 1;
        }
    }

    *fs_state.cancel.lock().await = None;
    emit_progress(
        &app,
        &format!("{downloaded} file(s) downloaded"),
        1.0,
        true,
        None,
    );
    Ok(downloaded)
}

// ---------------------------------------------------------------------------
// Upload commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn fs_upload(
    app: AppHandle,
    state: State<'_, HubState>,
    fs_state: State<'_, FsTransferState>,
    destination_dir: String,
) -> Result<(), String> {
    let file_path = app.dialog().file().blocking_pick_file();
    let file_path = match file_path {
        Some(p) => p.as_path().map(|p| p.to_path_buf()),
        None => return Ok(()),
    };
    let file_path = file_path.ok_or_else(|| "invalid file path".to_string())?;

    let name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "upload".to_string());

    let cancel = CancellationToken::new();
    *fs_state.cancel.lock().await = Some(cancel.clone());

    emit_progress(&app, &format!("Uploading {name}..."), 0.0, false, None);
    let result = upload_single_file_tcp(
        &app,
        &state,
        &file_path,
        &destination_dir,
        &name,
        0,
        0,
        &cancel,
    )
    .await;

    *fs_state.cancel.lock().await = None;
    match &result {
        Ok(()) => emit_progress(&app, &format!("{name} uploaded"), 1.0, true, None),
        Err(e) => emit_progress(&app, "Upload failed", 0.0, true, Some(e)),
    }
    result
}

#[tauri::command]
pub async fn fs_upload_local(
    app: AppHandle,
    state: State<'_, HubState>,
    fs_state: State<'_, FsTransferState>,
    paths: Vec<String>,
    destination_dir: String,
) -> Result<u32, String> {
    let total_bytes = count_local_bytes(&paths);
    let mut uploaded = 0u32;

    let cancel = CancellationToken::new();
    *fs_state.cancel.lock().await = Some(cancel.clone());
    let transferred = Arc::new(AtomicI64::new(0));

    emit_progress(&app, "Preparing upload...", 0.0, false, None);

    for path_str in &paths {
        if cancel.is_cancelled() {
            break;
        }
        let local_path = std::path::Path::new(path_str);
        if local_path.is_file() {
            let name = local_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let base = transferred.load(Ordering::Relaxed);
            upload_single_file_tcp(
                &app,
                &state,
                local_path,
                &destination_dir,
                &name,
                base,
                total_bytes,
                &cancel,
            )
            .await?;
            let size = std::fs::metadata(local_path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            transferred.fetch_add(size, Ordering::Relaxed);
            uploaded += 1;
        } else if local_path.is_dir() {
            uploaded += upload_directory_recursive(
                &app,
                &state,
                local_path,
                &destination_dir,
                &transferred,
                total_bytes,
                &cancel,
            )
            .await?;
        }
    }

    *fs_state.cancel.lock().await = None;
    emit_progress(
        &app,
        &format!("{uploaded} file(s) uploaded"),
        1.0,
        true,
        None,
    );
    Ok(uploaded)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn send_and_check<T: serde::Serialize>(
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

async fn agent_tcp_addr(state: &State<'_, HubState>, tcp_port: u16) -> Result<SocketAddr, String> {
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

fn count_local_bytes(paths: &[String]) -> i64 {
    let mut total = 0i64;
    for p in paths {
        let path = std::path::Path::new(p);
        if path.is_file() {
            total += std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
        } else if path.is_dir()
            && let Ok(entries) = std::fs::read_dir(path)
        {
            let sub: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path().to_string_lossy().into_owned())
                .collect();
            total += count_local_bytes(&sub);
        }
    }
    total
}

async fn count_remote_bytes(state: &State<'_, HubState>, remote_dir: &str) -> Result<i64, String> {
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

async fn get_remote_file_size(
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

// ---------------------------------------------------------------------------
// Upload via TCP
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
async fn upload_single_file_tcp(
    app: &AppHandle,
    state: &State<'_, HubState>,
    local_path: &std::path::Path,
    remote_dir: &str,
    name: &str,
    base_transferred: i64,
    total_bytes: i64,
    cancel: &CancellationToken,
) -> Result<(), String> {
    if cancel.is_cancelled() {
        return Err("cancelled".to_string());
    }

    let mgr = state.connection_mgr.clone();
    let file_size = std::fs::metadata(local_path)
        .map(|m| m.len() as i64)
        .map_err(|e| format!("failed to stat {}: {e}", local_path.display()))?;

    let req = FsUploadRequest {
        path: remote_dir.to_string(),
        name: name.to_string(),
        size: file_size,
    };
    let resp = mgr
        .send_request(MessageType::FsUpload, Some(&req))
        .await
        .map_err(|e| e.to_string())?;

    let ready = resp
        .parse_payload::<FsUploadReadyResponse>()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "empty upload ready response".to_string())?;

    let addr = agent_tcp_addr(state, ready.tcp_port).await?;
    let files = vec![(local_path.to_path_buf(), name.to_string())];
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(64);

    let app_clone = app.clone();
    let name_clone = name.to_string();
    let total = if total_bytes > 0 {
        total_bytes
    } else {
        file_size
    };
    let progress_task = tokio::spawn(async move {
        while let Some((bytes, _)) = progress_rx.recv().await {
            let done_bytes = base_transferred + bytes;
            let pct = (done_bytes as f64 / total as f64).min(0.99);
            let status = format!(
                "Uploading {}... {} / {}",
                name_clone,
                format_bytes(done_bytes),
                format_bytes(total)
            );
            emit_progress(&app_clone, &status, pct, false, None);
        }
    });

    let result = capydeploy_data_channel::client::TcpDataClient::connect_and_send(
        addr,
        &ready.tcp_token,
        &files,
        cancel.clone(),
        progress_tx,
    )
    .await
    .map_err(|e| format!("TCP upload failed: {e}"));

    let _ = progress_task.await;
    result?;
    Ok(())
}

async fn upload_directory_recursive(
    app: &AppHandle,
    state: &State<'_, HubState>,
    local_dir: &std::path::Path,
    remote_parent: &str,
    transferred: &Arc<AtomicI64>,
    total_bytes: i64,
    cancel: &CancellationToken,
) -> Result<u32, String> {
    if cancel.is_cancelled() {
        return Err("cancelled".to_string());
    }

    let dir_name = local_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| "invalid directory name".to_string())?;

    let remote_dir = format!("{remote_parent}/{dir_name}");
    send_and_check(
        state,
        MessageType::FsMkdir,
        Some(&FsMkdirRequest {
            path: remote_dir.clone(),
        }),
    )
    .await?;

    let mut count = 0u32;
    let entries = std::fs::read_dir(local_dir)
        .map_err(|e| format!("failed to read directory {}: {e}", local_dir.display()))?;

    for entry in entries {
        if cancel.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let entry_path = entry.path();
        if entry_path.is_file() {
            let name = entry_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let base = transferred.load(Ordering::Relaxed);
            upload_single_file_tcp(
                app,
                state,
                &entry_path,
                &remote_dir,
                &name,
                base,
                total_bytes,
                cancel,
            )
            .await?;
            let size = std::fs::metadata(&entry_path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            transferred.fetch_add(size, Ordering::Relaxed);
            count += 1;
        } else if entry_path.is_dir() {
            count += Box::pin(upload_directory_recursive(
                app,
                state,
                &entry_path,
                &remote_dir,
                transferred,
                total_bytes,
                cancel,
            ))
            .await?;
        }
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// Download via TCP
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
async fn download_single_file(
    app: &AppHandle,
    state: &State<'_, HubState>,
    remote_path: &str,
    local_file: &std::path::Path,
    base_transferred: i64,
    total_bytes: i64,
    cancel: &CancellationToken,
) -> Result<(), String> {
    if cancel.is_cancelled() {
        return Err("cancelled".to_string());
    }

    let mgr = state.connection_mgr.clone();
    let req = FsDownloadRequest {
        path: remote_path.to_string(),
    };
    let resp = mgr
        .send_request(MessageType::FsDownload, Some(&req))
        .await
        .map_err(|e| e.to_string())?;

    let download = resp
        .parse_payload::<FsDownloadReadyResponse>()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "empty download response".to_string())?;

    let file_size = download.size;
    let total = if total_bytes > 0 {
        total_bytes
    } else {
        file_size
    };
    let addr = agent_tcp_addr(state, download.tcp_port).await?;
    let output_dir = local_file
        .parent()
        .ok_or_else(|| "no parent directory".to_string())?;

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(64);

    let app_clone = app.clone();
    let file_name = local_file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let progress_task = tokio::spawn(async move {
        while let Some((bytes, _)) = progress_rx.recv().await {
            let done_bytes = base_transferred + bytes;
            let pct = (done_bytes as f64 / total as f64).min(0.99);
            let status = format!(
                "Downloading {}... {} / {}",
                file_name,
                format_bytes(done_bytes),
                format_bytes(total)
            );
            emit_progress(&app_clone, &status, pct, false, None);
        }
    });

    // TCP receiver writes to output_dir using the agent's filename.
    // We need to rename to the user's chosen filename afterwards.
    let agent_filename = std::path::Path::new(remote_path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "download".to_string());
    let tcp_received_path = output_dir.join(&agent_filename);

    let result = capydeploy_data_channel::client::TcpDataClient::connect_and_receive(
        addr,
        &download.tcp_token,
        output_dir,
        cancel.clone(),
        progress_tx,
    )
    .await
    .map_err(|e| format!("TCP download failed: {e}"));

    let _ = progress_task.await;

    if result.is_err() {
        let _ = std::fs::remove_file(&tcp_received_path);
        let _ = std::fs::remove_file(local_file);
    } else if tcp_received_path != local_file {
        // Rename from agent's filename to user's chosen filename.
        std::fs::rename(&tcp_received_path, local_file)
            .map_err(|e| format!("failed to rename downloaded file: {e}"))?;
    }
    result?;
    Ok(())
}

async fn download_directory_recursive(
    app: &AppHandle,
    state: &State<'_, HubState>,
    remote_dir: &str,
    local_dir: &std::path::Path,
    transferred: &Arc<AtomicI64>,
    total_bytes: i64,
    cancel: &CancellationToken,
) -> Result<u32, String> {
    if cancel.is_cancelled() {
        return Err("cancelled".to_string());
    }

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
        .ok_or_else(|| "empty fs_list response".to_string())?;

    let mut count = 0u32;
    for entry in &listing.entries {
        if cancel.is_cancelled() {
            return Err("cancelled".to_string());
        }
        if entry.is_dir {
            let sub_local = local_dir.join(&entry.name);
            std::fs::create_dir_all(&sub_local)
                .map_err(|e| format!("failed to create {}: {e}", sub_local.display()))?;
            count += Box::pin(download_directory_recursive(
                app,
                state,
                &entry.path,
                &sub_local,
                transferred,
                total_bytes,
                cancel,
            ))
            .await?;
        } else {
            let base = transferred.load(Ordering::Relaxed);
            let local_file = local_dir.join(&entry.name);
            download_single_file(
                app,
                state,
                &entry.path,
                &local_file,
                base,
                total_bytes,
                cancel,
            )
            .await?;
            transferred.fetch_add(entry.size, Ordering::Relaxed);
            count += 1;
        }
    }
    Ok(count)
}
