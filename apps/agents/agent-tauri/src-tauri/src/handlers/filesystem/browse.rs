//! Browse handlers: fs_list, fs_mkdir, fs_delete, fs_rename.

use std::path::Path;
use std::time::UNIX_EPOCH;

use capydeploy_agent_server::Sender;
use capydeploy_protocol::constants::{self, MessageType};
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::handler::TauriAgentHandler;

impl TauriAgentHandler {
    pub(crate) async fn handle_fs_list(&self, sender: Sender, msg: Message) {
        let req: messages::FsListRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ =
                    sender.send_error(&msg, constants::WS_ERR_CODE_BAD_REQUEST, "invalid payload");
                return;
            }
        };

        let canonical = match self.validate_path(&req.path, &msg, &sender) {
            Some(p) => p,
            None => return,
        };

        let show_hidden = req.show_hidden;
        let result =
            tokio::task::spawn_blocking(move || read_dir_entries(&canonical, show_hidden)).await;

        match result {
            Ok(Ok((entries, truncated))) => {
                let resp = messages::FsListResponse {
                    path: req.path,
                    entries,
                    truncated,
                };
                if let Ok(reply) = msg.reply(MessageType::FsListResponse, Some(&resp)) {
                    let _ = sender.send_msg(reply);
                }
            }
            Ok(Err(e)) => {
                let _ = sender.send_error(&msg, constants::WS_ERR_CODE_INTERNAL, &e);
            }
            Err(e) => {
                tracing::error!("fs_list task panicked: {e}");
                let _ = sender.send_error(&msg, constants::WS_ERR_CODE_INTERNAL, "internal error");
            }
        }
    }

    pub(crate) async fn handle_fs_mkdir(&self, sender: Sender, msg: Message) {
        let req: messages::FsMkdirRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ =
                    sender.send_error(&msg, constants::WS_ERR_CODE_BAD_REQUEST, "invalid payload");
                return;
            }
        };

        let target = match self.validate_new_path(&req.path, &msg, &sender) {
            Some(p) => p,
            None => return,
        };

        let result = tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&target).map_err(|e| format!("failed to create directory: {e}"))
        })
        .await;

        reply_operation_result(&sender, &msg, result);
    }

    pub(crate) async fn handle_fs_delete(&self, sender: Sender, msg: Message) {
        let req: messages::FsDeleteRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ =
                    sender.send_error(&msg, constants::WS_ERR_CODE_BAD_REQUEST, "invalid payload");
                return;
            }
        };

        let canonical = match self.validate_path(&req.path, &msg, &sender) {
            Some(p) => p,
            None => return,
        };

        let result = tokio::task::spawn_blocking(move || {
            if canonical.is_dir() {
                std::fs::remove_dir_all(&canonical)
            } else {
                std::fs::remove_file(&canonical)
            }
            .map_err(|e| format!("failed to delete: {e}"))
        })
        .await;

        reply_operation_result(&sender, &msg, result);
    }

    pub(crate) async fn handle_fs_rename(&self, sender: Sender, msg: Message) {
        let req: messages::FsRenameRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ =
                    sender.send_error(&msg, constants::WS_ERR_CODE_BAD_REQUEST, "invalid payload");
                return;
            }
        };

        let old = match self.validate_path(&req.old_path, &msg, &sender) {
            Some(p) => p,
            None => return,
        };
        let new = match self.validate_new_path(&req.new_path, &msg, &sender) {
            Some(p) => p,
            None => return,
        };

        let result = tokio::task::spawn_blocking(move || {
            std::fs::rename(&old, &new).map_err(|e| format!("failed to rename: {e}"))
        })
        .await;

        reply_operation_result(&sender, &msg, result);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Send an OperationResult reply from a spawn_blocking result.
fn reply_operation_result(
    sender: &Sender,
    msg: &Message,
    result: Result<Result<(), String>, tokio::task::JoinError>,
) {
    match result {
        Ok(Ok(())) => {
            let resp = messages::OperationResult {
                success: true,
                message: String::new(),
            };
            if let Ok(reply) = msg.reply(MessageType::OperationResult, Some(&resp)) {
                let _ = sender.send_msg(reply);
            }
        }
        Ok(Err(e)) => {
            let _ = sender.send_error(msg, constants::WS_ERR_CODE_INTERNAL, &e);
        }
        Err(e) => {
            tracing::error!("task panicked: {e}");
            let _ = sender.send_error(msg, constants::WS_ERR_CODE_INTERNAL, "internal error");
        }
    }
}

fn read_dir_entries(
    path: &Path,
    show_hidden: bool,
) -> Result<(Vec<messages::FsEntry>, bool), String> {
    let read_dir =
        std::fs::read_dir(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    let mut entries = Vec::new();
    let mut truncated = false;

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let mod_time = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);

        entries.push(messages::FsEntry {
            name,
            path: entry.path().to_string_lossy().into_owned(),
            is_dir: metadata.is_dir(),
            size: if metadata.is_dir() {
                0
            } else {
                metadata.len() as i64
            },
            mod_time,
            is_symlink,
        });

        if entries.len() >= constants::FS_MAX_LIST_ENTRIES {
            truncated = true;
            break;
        }
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok((entries, truncated))
}
