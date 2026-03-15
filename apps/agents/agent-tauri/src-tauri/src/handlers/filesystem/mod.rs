//! Filesystem browsing handlers — sandbox, browse, and TCP transfers.

mod browse;
mod transfer;

use std::path::{Path, PathBuf};

use capydeploy_agent_server::Sender;
use capydeploy_protocol::constants;
use capydeploy_protocol::envelope::Message;

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

    /// Validates a path that may not exist yet (parent must be in sandbox).
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
// Shared helpers
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

impl TauriAgentHandler {
    /// Validate + resolve a path string against the sandbox.
    pub(crate) fn validate_path(
        &self,
        raw: &str,
        msg: &Message,
        sender: &Sender,
    ) -> Option<PathBuf> {
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

    /// Validate a path whose target may not exist yet.
    pub(crate) fn validate_new_path(
        &self,
        raw: &str,
        msg: &Message,
        sender: &Sender,
    ) -> Option<PathBuf> {
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
}
