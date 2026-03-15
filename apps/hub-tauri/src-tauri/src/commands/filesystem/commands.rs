//! Read-only filesystem commands + cancel.

use tauri::State;

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::messages::{
    FsDeleteRequest, FsListRequest, FsListResponse, FsMkdirRequest, FsRenameRequest,
};

use crate::state::HubState;

use super::{FsTransferState, send_and_check};

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

#[tauri::command]
pub async fn fs_cancel_transfer(fs_state: State<'_, FsTransferState>) -> Result<(), String> {
    let mut guard = fs_state.cancel.lock().await;
    if let Some(token) = guard.take() {
        token.cancel();
    }
    Ok(())
}
