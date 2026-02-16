//! Game log wrapper: inject/strip launch options, tail log files, stream to Hub.
//!
//! Linux-only. Injects a bash wrapper script into Steam launch options that
//! captures game stdout/stderr to a log file. The log tailer watches these
//! files and streams entries to the Hub.

#[cfg(target_os = "linux")]
mod tailer;
#[cfg(target_os = "linux")]
mod wrapper;

#[cfg(target_os = "linux")]
pub use tailer::{LogTailer, find_latest_log};
#[cfg(target_os = "linux")]
pub use wrapper::{WrapperManager, log_dir, log_file_pattern};

/// The embedded wrapper script content.
pub const WRAPPER_SCRIPT: &str = include_str!("wrapper.sh");

/// Default log directory under `$HOME/.local/share/capydeploy/logs/`.
pub const LOG_DIR_NAME: &str = "capydeploy/logs";
