//! Tray handle, events, and update types.
//!
//! The actual system tray implementation depends on `tray-icon` and `muda`
//! crates, which require platform-specific system libraries. This module
//! defines the channel-based interface that the agent core uses to
//! communicate with the tray, independent of the GUI backend.

use std::sync::mpsc;

use crate::menu::MenuState;

/// Configuration for the system tray.
#[derive(Debug, Clone)]
pub struct TrayConfig {
    /// Agent display name shown in the tray tooltip.
    pub agent_name: String,
    /// Optional icon data (PNG bytes).
    pub icon_data: Option<Vec<u8>>,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            agent_name: "CapyDeploy Agent".into(),
            icon_data: None,
        }
    }
}

/// Events emitted by the tray to the agent core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayEvent {
    /// User clicked "Quit" in the context menu.
    QuitRequested,
}

/// Updates sent from the agent core to the tray.
#[derive(Debug, Clone)]
pub enum TrayUpdate {
    /// A Hub connected.
    HubConnected(String),
    /// A Hub disconnected.
    HubDisconnected(String),
    /// Agent running state changed.
    RunningStateChanged(bool),
    /// Request tray shutdown.
    Shutdown,
}

/// Handle for communicating with the system tray from the agent core.
///
/// This is the async-safe side of the tray interface. The tray event loop
/// runs on the main thread and communicates via channels.
pub struct TrayHandle {
    /// Send updates to the tray.
    update_tx: mpsc::Sender<TrayUpdate>,
    /// Receive events from the tray.
    event_rx: mpsc::Receiver<TrayEvent>,
    /// Current menu state (for tracking).
    state: MenuState,
}

impl TrayHandle {
    /// Creates a new tray handle with its channel pair.
    ///
    /// Returns `(handle, event_sender, update_receiver)` — the sender/receiver
    /// pair is given to the tray event loop running on the main thread.
    pub fn new(config: TrayConfig) -> (Self, mpsc::Sender<TrayEvent>, mpsc::Receiver<TrayUpdate>) {
        let (update_tx, update_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        let handle = Self {
            update_tx,
            event_rx,
            state: MenuState {
                agent_name: config.agent_name,
                running: true,
                connected_hubs: Vec::new(),
            },
        };

        (handle, event_tx, update_rx)
    }

    /// Notifies the tray that a Hub has connected.
    pub fn hub_connected(&mut self, name: String) {
        if !self.state.connected_hubs.contains(&name) {
            self.state.connected_hubs.push(name.clone());
        }
        let _ = self.update_tx.send(TrayUpdate::HubConnected(name));
    }

    /// Notifies the tray that a Hub has disconnected.
    pub fn hub_disconnected(&mut self, name: &str) {
        self.state.connected_hubs.retain(|h| h != name);
        let _ = self
            .update_tx
            .send(TrayUpdate::HubDisconnected(name.to_string()));
    }

    /// Updates the running state.
    pub fn set_running(&mut self, running: bool) {
        self.state.running = running;
        let _ = self
            .update_tx
            .send(TrayUpdate::RunningStateChanged(running));
    }

    /// Requests the tray to shut down.
    pub fn shutdown(&self) {
        let _ = self.update_tx.send(TrayUpdate::Shutdown);
    }

    /// Tries to receive a tray event (non-blocking).
    pub fn try_recv_event(&self) -> Option<TrayEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Returns the current menu state.
    pub fn state(&self) -> &MenuState {
        &self.state
    }

    /// Returns the number of connected Hubs.
    pub fn connected_hub_count(&self) -> usize {
        self.state.connected_hubs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_handle_creation() {
        let config = TrayConfig::default();
        let (handle, _event_tx, _update_rx) = TrayHandle::new(config);
        assert_eq!(handle.state().agent_name, "CapyDeploy Agent");
        assert!(handle.state().running);
        assert_eq!(handle.connected_hub_count(), 0);
    }

    #[test]
    fn tray_handle_hub_tracking() {
        let (mut handle, _event_tx, update_rx) = TrayHandle::new(TrayConfig::default());

        handle.hub_connected("Hub-1".into());
        assert_eq!(handle.connected_hub_count(), 1);

        // Duplicate connect should not add twice.
        handle.hub_connected("Hub-1".into());
        assert_eq!(handle.connected_hub_count(), 1);

        handle.hub_connected("Hub-2".into());
        assert_eq!(handle.connected_hub_count(), 2);

        handle.hub_disconnected("Hub-1");
        assert_eq!(handle.connected_hub_count(), 1);

        // Verify updates were sent.
        let mut updates = Vec::new();
        while let Ok(u) = update_rx.try_recv() {
            updates.push(u);
        }
        assert!(updates.len() >= 3); // 2 connects + 1 disconnect (duplicate also sends).
    }

    #[test]
    fn tray_handle_running_state() {
        let (mut handle, _event_tx, _update_rx) = TrayHandle::new(TrayConfig::default());

        assert!(handle.state().running);
        handle.set_running(false);
        assert!(!handle.state().running);
    }

    #[test]
    fn tray_handle_events() {
        let (handle, event_tx, _update_rx) = TrayHandle::new(TrayConfig::default());

        // No events yet.
        assert!(handle.try_recv_event().is_none());

        // Send a quit event from the tray.
        event_tx.send(TrayEvent::QuitRequested).unwrap();
        let event = handle.try_recv_event().unwrap();
        assert_eq!(event, TrayEvent::QuitRequested);
    }

    #[test]
    fn tray_handle_shutdown() {
        let (handle, _event_tx, update_rx) = TrayHandle::new(TrayConfig::default());

        handle.shutdown();
        let update = update_rx.recv().unwrap();
        assert!(matches!(update, TrayUpdate::Shutdown));
    }

    #[test]
    fn tray_config_default() {
        let config = TrayConfig::default();
        assert_eq!(config.agent_name, "CapyDeploy Agent");
        assert!(config.icon_data.is_none());
    }

    #[test]
    fn tray_event_equality() {
        assert_eq!(TrayEvent::QuitRequested, TrayEvent::QuitRequested);
    }
}
