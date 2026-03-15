//! Upload commands and TCP upload helpers.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::messages::{FsMkdirRequest, FsUploadReadyResponse, FsUploadRequest};

use crate::state::HubState;

use super::{FsTransferState, agent_tcp_addr, emit_progress, format_bytes, send_and_check};

// ---------------------------------------------------------------------------
// Commands
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
// TCP upload helpers
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
