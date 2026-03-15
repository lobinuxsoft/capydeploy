//! Download commands and TCP download helpers.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::messages::{
    FsDownloadReadyResponse, FsDownloadRequest, FsListRequest, FsListResponse,
};

use crate::state::HubState;

use super::{
    FsTransferState, agent_tcp_addr, count_remote_bytes, emit_progress, format_bytes,
    get_remote_file_size,
};

// ---------------------------------------------------------------------------
// Commands
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
// TCP download helpers
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
    // We rename to the user's chosen filename afterwards.
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
