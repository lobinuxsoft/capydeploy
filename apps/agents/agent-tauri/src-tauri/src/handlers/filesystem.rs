use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use capydeploy_agent_server::Sender;
use capydeploy_protocol::constants::{self, MessageType};
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::handler::TauriAgentHandler;

// ---------------------------------------------------------------------------
// Path sandbox
// ---------------------------------------------------------------------------

/// Restricts filesystem access to a set of allowed root directories.
pub struct FsSandbox {
    allowed_roots: Vec<PathBuf>,
}

impl FsSandbox {
    /// Creates a sandbox with platform-appropriate default roots.
    pub fn default_roots() -> Self {
        let mut roots = Vec::new();

        if let Some(home) = home_dir() {
            roots.push(home);
        }

        #[cfg(target_os = "linux")]
        {
            let media = PathBuf::from("/run/media");
            if media.exists() {
                roots.push(media);
            }
        }

        #[cfg(target_os = "windows")]
        {
            for letter in b'A'..=b'Z' {
                let drive = PathBuf::from(format!("{}:\\", letter as char));
                if drive.exists() {
                    roots.push(drive);
                }
            }
        }

        Self {
            allowed_roots: roots,
        }
    }

    /// Returns `Ok(absolute_path)` if the path is within an allowed root.
    ///
    /// Resolves `..` traversals without following symlinks, then checks
    /// the result against allowed roots. This avoids issues on immutable
    /// distros where `/home` → `/var/home`.
    pub fn validate(&self, path: &Path) -> Result<PathBuf, String> {
        let absolute = std::path::absolute(path)
            .map_err(|e| format!("failed to resolve {}: {e}", path.display()))?;

        for root in &self.allowed_roots {
            if absolute.starts_with(root) {
                return Ok(absolute);
            }
        }

        Err(format!(
            "access denied: {} is outside allowed roots",
            absolute.display()
        ))
    }

    /// Validates a path that may not exist yet (for mkdir/upload).
    /// Checks that the *parent* directory is within an allowed root.
    pub fn validate_parent(&self, path: &Path) -> Result<PathBuf, String> {
        let absolute = std::path::absolute(path)
            .map_err(|e| format!("failed to resolve {}: {e}", path.display()))?;
        let parent = absolute
            .parent()
            .ok_or_else(|| format!("no parent directory for {}", path.display()))?;

        for root in &self.allowed_roots {
            if parent.starts_with(root) {
                return Ok(absolute);
            }
        }

        Err(format!(
            "access denied: {} is outside allowed roots",
            path.display()
        ))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolves `~` prefix to the user's home directory.
fn resolve_home(raw: &str) -> Option<PathBuf> {
    if raw == "~" {
        home_dir()
    } else if let Some(rest) = raw.strip_prefix("~/") {
        home_dir().map(|h| h.join(rest))
    } else {
        Some(PathBuf::from(raw))
    }
}

/// Cross-platform home directory resolution.
fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

impl TauriAgentHandler {
    /// Validate + resolve a path string against the sandbox.
    fn validate_path(&self, raw: &str, msg: &Message, sender: &Sender) -> Option<PathBuf> {
        let path = match resolve_home(raw) {
            Some(p) => p,
            None => {
                let _ = sender.send_error(
                    msg,
                    constants::WS_ERR_CODE_INTERNAL,
                    "cannot resolve home directory",
                );
                return None;
            }
        };
        match self.state.fs_sandbox.validate(&path) {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!("sandbox denied: {e}");
                let _ = sender.send_error(msg, constants::WS_ERR_CODE_BAD_REQUEST, &e);
                None
            }
        }
    }

    /// Validate a path whose target may not exist yet (parent must exist).
    fn validate_new_path(&self, raw: &str, msg: &Message, sender: &Sender) -> Option<PathBuf> {
        let path = match resolve_home(raw) {
            Some(p) => p,
            None => {
                let _ = sender.send_error(
                    msg,
                    constants::WS_ERR_CODE_INTERNAL,
                    "cannot resolve home directory",
                );
                return None;
            }
        };
        match self.state.fs_sandbox.validate_parent(&path) {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!("sandbox denied: {e}");
                let _ = sender.send_error(msg, constants::WS_ERR_CODE_BAD_REQUEST, &e);
                None
            }
        }
    }

    // -- fs_list ---------------------------------------------------------------

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

    // -- fs_mkdir ---------------------------------------------------------------

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
                let _ = sender.send_error(&msg, constants::WS_ERR_CODE_INTERNAL, &e);
            }
            Err(e) => {
                tracing::error!("fs_mkdir task panicked: {e}");
                let _ = sender.send_error(&msg, constants::WS_ERR_CODE_INTERNAL, "internal error");
            }
        }
    }

    // -- fs_delete --------------------------------------------------------------

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
                let _ = sender.send_error(&msg, constants::WS_ERR_CODE_INTERNAL, &e);
            }
            Err(e) => {
                tracing::error!("fs_delete task panicked: {e}");
                let _ = sender.send_error(&msg, constants::WS_ERR_CODE_INTERNAL, "internal error");
            }
        }
    }

    // -- fs_rename --------------------------------------------------------------

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
                let _ = sender.send_error(&msg, constants::WS_ERR_CODE_INTERNAL, &e);
            }
            Err(e) => {
                tracing::error!("fs_rename task panicked: {e}");
                let _ = sender.send_error(&msg, constants::WS_ERR_CODE_INTERNAL, "internal error");
            }
        }
    }

    // -- fs_download (always TCP) ------------------------------------------------

    pub(crate) async fn handle_fs_download(&self, sender: Sender, msg: Message) {
        let req: messages::FsDownloadRequest = match msg.parse_payload() {
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

        let meta = match tokio::fs::metadata(&canonical).await {
            Ok(m) => m,
            Err(e) => {
                let _ = sender.send_error(
                    &msg,
                    constants::WS_ERR_CODE_BAD_REQUEST,
                    &format!("cannot stat file: {e}"),
                );
                return;
            }
        };

        if meta.is_dir() {
            let _ = sender.send_error(
                &msg,
                constants::WS_ERR_CODE_BAD_REQUEST,
                "cannot download a directory",
            );
            return;
        }

        let file_size = meta.len();
        let cancel = self.state.shutdown_token.clone();

        let listener = match tokio::net::TcpListener::bind("0.0.0.0:0").await {
            Ok(l) => l,
            Err(e) => {
                let _ = sender.send_error(
                    &msg,
                    constants::WS_ERR_CODE_INTERNAL,
                    &format!("failed to bind TCP: {e}"),
                );
                return;
            }
        };
        let tcp_port = listener.local_addr().unwrap().port();
        let tcp_token = capydeploy_data_channel::generate_token();

        let resp = messages::FsDownloadReadyResponse {
            path: canonical.to_string_lossy().into_owned(),
            size: file_size as i64,
            tcp_port,
            tcp_token: tcp_token.clone(),
        };
        if let Ok(reply) = msg.reply(MessageType::FsDownloadReady, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }

        let file_name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "download".to_string());
        let files = vec![(canonical, file_name)];
        let (progress_tx, _) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            match capydeploy_data_channel::server::TcpDataServer::accept_and_send(
                listener,
                &tcp_token,
                &files,
                cancel,
                progress_tx,
            )
            .await
            {
                Ok(bytes) => tracing::info!(bytes, "fs_download TCP complete"),
                Err(e) => tracing::error!("fs_download TCP failed: {e}"),
            }
        });
    }

    // -- fs_upload (always TCP) -------------------------------------------------

    pub(crate) async fn handle_fs_upload(&self, sender: Sender, msg: Message) {
        let req: messages::FsUploadRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ =
                    sender.send_error(&msg, constants::WS_ERR_CODE_BAD_REQUEST, "invalid payload");
                return;
            }
        };

        let dir = match self.validate_path(&req.path, &msg, &sender) {
            Some(p) => p,
            None => return,
        };

        let cancel = self.state.shutdown_token.clone();
        let base_path = dir.clone();

        let tcp_server = capydeploy_data_channel::server::TcpDataServer::new(base_path, cancel);
        let (info, listener) = match tcp_server.listen().await {
            Ok(r) => r,
            Err(e) => {
                let _ = sender.send_error(
                    &msg,
                    constants::WS_ERR_CODE_INTERNAL,
                    &format!("failed to start TCP: {e}"),
                );
                return;
            }
        };

        let resp = messages::FsUploadReadyResponse {
            tcp_port: info.port,
            tcp_token: info.token.clone(),
        };
        if let Ok(reply) = msg.reply(MessageType::FsUploadReady, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }

        let token = info.token;
        let (progress_tx, _) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            match tcp_server
                .accept_and_receive(listener, &token, progress_tx)
                .await
            {
                Ok(bytes) => tracing::info!(bytes, "fs_upload TCP complete"),
                Err(e) => tracing::error!("fs_upload TCP failed: {e}"),
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Directory listing helper
// ---------------------------------------------------------------------------

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
