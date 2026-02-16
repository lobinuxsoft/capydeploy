//! System tray icon for the CapyDeploy headless agent.
//!
//! Provides a cross-platform tray icon with a context menu showing
//! connection status, connected Hubs, and a quit action.
//!
//! The tray communicates with the agent core via channels:
//! - [`TrayEvent`] — events from tray to agent (e.g. quit requested)
//! - [`TrayUpdate`] — updates from agent to tray (e.g. connection status change)
//!
//! # Platform notes
//! - Linux: Uses StatusNotifierItem (Wayland) or X11 tray protocol
//! - Windows: Uses Win32 Shell_NotifyIcon
//! - The tray event loop must run on the main thread on some platforms

mod menu;
mod tray;

pub use menu::{MenuAction, MenuItem, MenuState};
pub use tray::{TrayConfig, TrayEvent, TrayHandle, TrayUpdate};
