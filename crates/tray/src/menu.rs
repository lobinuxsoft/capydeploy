//! Dynamic context menu for the system tray.

/// Actions that can be triggered from the tray context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    /// User requested to quit the application.
    Quit,
}

/// A single menu item.
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Display text.
    pub label: String,
    /// Whether the item is enabled (clickable).
    pub enabled: bool,
    /// Optional action triggered on click.
    pub action: Option<MenuAction>,
}

/// Current state used to build the context menu.
#[derive(Debug, Clone)]
pub struct MenuState {
    /// Agent display name.
    pub agent_name: String,
    /// Whether the agent is running.
    pub running: bool,
    /// Names of connected Hubs.
    pub connected_hubs: Vec<String>,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            agent_name: "CapyDeploy Agent".into(),
            running: true,
            connected_hubs: Vec::new(),
        }
    }
}

impl MenuState {
    /// Builds the menu items from the current state.
    pub fn build_menu(&self) -> Vec<MenuItem> {
        let mut items = Vec::new();

        // Header: agent name + status.
        let status = if self.running { "Running" } else { "Stopped" };
        items.push(MenuItem {
            label: format!("{} — {status}", self.agent_name),
            enabled: false,
            action: None,
        });

        // Separator (represented as disabled empty item).
        items.push(MenuItem {
            label: String::new(),
            enabled: false,
            action: None,
        });

        // Connected Hubs.
        if self.connected_hubs.is_empty() {
            items.push(MenuItem {
                label: "No Hubs connected".into(),
                enabled: false,
                action: None,
            });
        } else {
            items.push(MenuItem {
                label: format!("Hubs ({}):", self.connected_hubs.len()),
                enabled: false,
                action: None,
            });
            for hub in &self.connected_hubs {
                items.push(MenuItem {
                    label: format!("  {hub}"),
                    enabled: false,
                    action: None,
                });
            }
        }

        // Separator.
        items.push(MenuItem {
            label: String::new(),
            enabled: false,
            action: None,
        });

        // Quit.
        items.push(MenuItem {
            label: "Quit".into(),
            enabled: true,
            action: Some(MenuAction::Quit),
        });

        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_menu_state() {
        let state = MenuState::default();
        assert_eq!(state.agent_name, "CapyDeploy Agent");
        assert!(state.running);
        assert!(state.connected_hubs.is_empty());
    }

    #[test]
    fn build_menu_no_hubs() {
        let state = MenuState::default();
        let items = state.build_menu();

        // Should have: header, separator, "No Hubs", separator, quit.
        assert!(items.len() >= 4);
        assert!(items[0].label.contains("Running"));
        assert!(items.iter().any(|i| i.label == "No Hubs connected"));
        assert!(items.last().unwrap().action == Some(MenuAction::Quit));
    }

    #[test]
    fn build_menu_with_hubs() {
        let state = MenuState {
            agent_name: "Test Agent".into(),
            running: true,
            connected_hubs: vec!["Hub-1".into(), "Hub-2".into()],
        };
        let items = state.build_menu();

        assert!(items[0].label.contains("Test Agent"));
        assert!(items.iter().any(|i| i.label.contains("Hubs (2)")));
        assert!(items.iter().any(|i| i.label.contains("Hub-1")));
        assert!(items.iter().any(|i| i.label.contains("Hub-2")));
    }

    #[test]
    fn build_menu_stopped_status() {
        let state = MenuState {
            running: false,
            ..MenuState::default()
        };
        let items = state.build_menu();
        assert!(items[0].label.contains("Stopped"));
    }

    #[test]
    fn quit_item_is_enabled() {
        let items = MenuState::default().build_menu();
        let quit = items.iter().find(|i| i.action == Some(MenuAction::Quit));
        assert!(quit.is_some());
        assert!(quit.unwrap().enabled);
    }

    #[test]
    fn menu_action_equality() {
        assert_eq!(MenuAction::Quit, MenuAction::Quit);
    }
}
