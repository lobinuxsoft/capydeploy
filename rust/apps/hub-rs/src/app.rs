//! Hub application — `cosmic::Application` implementation.

use std::collections::HashMap;
use std::sync::Arc;

use cosmic::app::Core;
use cosmic::iced::widget::container as iced_container;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, container};
use cosmic::{Application, Element};

use capydeploy_discovery::types::DiscoveredAgent;
use capydeploy_hub_connection::pairing::default_token_path;
use capydeploy_hub_connection::{
    ConnectedAgent, ConnectionEvent, ConnectionManager, ConnectionState, HubIdentity, TokenStore,
};
use capydeploy_hub_console_log::ConsoleLogHub;
use capydeploy_hub_telemetry::TelemetryHub;
use capydeploy_protocol::constants::{self, MessageType};

use crate::config::HubConfig;
use crate::dialogs::pairing::PairingDialog;
use crate::message::{Message, NavPage};
use crate::theme;
use crate::views::TelemetryWidgets;

/// Main Hub application state.
pub struct Hub {
    core: Core,
    config: HubConfig,
    nav_page: NavPage,

    // Connection state.
    connection_mgr: Arc<ConnectionManager>,
    discovered_agents: Vec<DiscoveredAgent>,
    connected_agent: Option<ConnectedAgent>,
    connection_states: HashMap<String, ConnectionState>,

    // Pairing dialog.
    pairing_dialog: Option<PairingDialog>,

    // Telemetry state.
    telemetry_hub: TelemetryHub,
    telemetry_widgets: TelemetryWidgets,

    // Console log state.
    console_log_hub: ConsoleLogHub,
    console_level_filter: u32,
    console_search: String,
}

impl Hub {
    /// Whether any agent is currently connected.
    fn is_connected(&self) -> bool {
        self.connected_agent.is_some()
    }

    /// The connected agent's ID, if any.
    fn connected_agent_id(&self) -> Option<&str> {
        self.connected_agent
            .as_ref()
            .map(|a| a.agent.info.id.as_str())
    }

    /// Handles setting up the connected state after a successful connection.
    fn on_connected(&mut self, agent: ConnectedAgent) {
        let id = agent.agent.info.id.clone();
        tracing::info!(agent = %id, name = %agent.agent.info.name, "connected to agent");
        self.connection_states
            .insert(id, ConnectionState::Connected);
        self.connected_agent = Some(agent);
        self.pairing_dialog = None;
    }
}

impl Application for Hub {
    type Executor = cosmic::executor::Default;
    type Message = Message;
    type Flags = HubConfig;

    const APP_ID: &'static str = "com.capydeploy.hub";

    fn init(mut core: Core, config: HubConfig) -> (Self, cosmic::app::Task<Message>) {
        core.window.show_headerbar = false;

        let hub_identity = HubIdentity {
            name: config.name.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            platform: std::env::consts::OS.to_string(),
            hub_id: config.hub_id.clone(),
        };

        // Load token store for pairing persistence.
        let token_store = default_token_path()
            .and_then(|path| {
                TokenStore::new(path)
                    .inspect_err(|e| tracing::warn!(error = %e, "failed to load token store"))
                    .ok()
            })
            .map(Arc::new);

        let connection_mgr = Arc::new(ConnectionManager::new(hub_identity, token_store));

        let app = Self {
            core,
            config,
            nav_page: NavPage::Devices,
            connection_mgr,
            discovered_agents: Vec::new(),
            connected_agent: None,
            connection_states: HashMap::new(),
            pairing_dialog: None,
            telemetry_hub: TelemetryHub::new(),
            telemetry_widgets: TelemetryWidgets::new(),
            console_log_hub: ConsoleLogHub::new(),
            console_level_filter: constants::LOG_LEVEL_DEFAULT | constants::LOG_LEVEL_DEBUG,
            console_search: String::new(),
        };

        (app, cosmic::app::Task::none())
    }

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Message> {
        crate::subscriptions::connection_events(self.connection_mgr.clone())
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        match message {
            // -- Navigation --
            Message::NavigateTo(page) => {
                if !page.requires_connection() || self.is_connected() {
                    self.nav_page = page;
                }
            }

            // -- Connection lifecycle --
            Message::DiscoveryStarted => {
                tracing::debug!("discovery subscription active");
            }

            Message::ConnectionEvent(event) => {
                return self.handle_connection_event(event);
            }

            Message::ConnectAgent(agent_id) => {
                self.connection_states
                    .insert(agent_id.clone(), ConnectionState::Connecting);
                let mgr = self.connection_mgr.clone();
                return cosmic::app::Task::perform(
                    async move { mgr.connect_agent(&agent_id).await },
                    |result| cosmic::action::app(Message::ConnectResult(result.map_err(|e| e.to_string()))),
                );
            }

            Message::ConnectResult(result) => match result {
                Ok(agent) => {
                    self.on_connected(agent);
                }
                Err(e) => {
                    // Pairing-required errors are handled via ConnectionEvent::PairingNeeded.
                    if !e.contains("pairing required") {
                        tracing::warn!(error = %e, "connection failed");
                    }
                }
            },

            Message::DisconnectAgent => {
                if let Some(agent) = self.connected_agent.take() {
                    let id = agent.agent.info.id.clone();
                    self.connection_states
                        .insert(id, ConnectionState::Disconnected);
                }
                // Fall back to Devices page if on a connection-dependent page.
                if self.nav_page.requires_connection() {
                    self.nav_page = NavPage::Devices;
                }
                let mgr = self.connection_mgr.clone();
                return cosmic::app::Task::perform(
                    async move { mgr.disconnect_agent().await },
                    |()| cosmic::action::app(Message::DiscoveryStarted),
                );
            }

            // -- Pairing --
            Message::PairingCodeInput(input) => {
                if let Some(dialog) = &mut self.pairing_dialog {
                    dialog.input = input;
                }
            }

            Message::ConfirmPairing => {
                if let Some(dialog) = &mut self.pairing_dialog {
                    dialog.confirming = true;
                    let mgr = self.connection_mgr.clone();
                    let agent_id = dialog.agent_id.clone();
                    let code = dialog.input.clone();
                    return cosmic::app::Task::perform(
                        async move { mgr.confirm_pairing(&agent_id, &code).await },
                        |result| cosmic::action::app(Message::PairingResult(result.map_err(|e| e.to_string()))),
                    );
                }
            }

            Message::CancelPairing => {
                if let Some(dialog) = self.pairing_dialog.take() {
                    self.connection_states
                        .insert(dialog.agent_id, ConnectionState::Discovered);
                    let mgr = self.connection_mgr.clone();
                    return cosmic::app::Task::perform(
                        async move { mgr.disconnect_agent().await },
                        |()| cosmic::action::app(Message::DiscoveryStarted),
                    );
                }
            }

            Message::PairingResult(result) => match result {
                Ok(agent) => {
                    self.on_connected(agent);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "pairing failed");
                    if let Some(dialog) = &mut self.pairing_dialog {
                        dialog.confirming = false;
                        dialog.input.clear();
                    }
                }
            },

            // -- Console Log --
            Message::ConsoleToggleLevel(bit) => {
                self.console_level_filter ^= bit;
            }

            Message::ConsoleSearchInput(text) => {
                self.console_search = text;
            }

            Message::ConsoleClear => {
                if let Some(id) = self.connected_agent_id().map(String::from) {
                    self.console_log_hub.remove_agent(&id);
                }
            }

            Message::ConsoleSetEnabled(enabled) => {
                let mgr = self.connection_mgr.clone();
                let payload = serde_json::json!({ "enabled": enabled });
                return cosmic::app::Task::perform(
                    async move {
                        mgr.send_request(MessageType::SetConsoleLogEnabled, Some(&payload))
                            .await
                    },
                    move |result| {
                        cosmic::action::app(Message::ConsoleSetEnabledResult(
                            result
                                .map(|_| enabled)
                                .map_err(|e| e.to_string()),
                        ))
                    },
                );
            }

            Message::ConsoleSetEnabledResult(result) => match result {
                Ok(enabled) => {
                    tracing::info!(enabled, "console log streaming toggled");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to toggle console log");
                }
            },

            // -- System --
            Message::Tick => {}
        }
        cosmic::app::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();
        let content = self.view_content();

        let main: Element<'_, Message> = widget::row()
            .push(sidebar)
            .push(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        // Overlay pairing dialog if active.
        if let Some(dialog) = &self.pairing_dialog {
            let overlay = crate::dialogs::pairing::view(dialog);
            cosmic::widget::popover(main)
                .modal(true)
                .popup(overlay)
                .on_close(Message::CancelPairing)
                .into()
        } else {
            main
        }
    }
}

// ---------------------------------------------------------------------------
// Connection event routing
// ---------------------------------------------------------------------------

impl Hub {
    fn handle_connection_event(&mut self, event: ConnectionEvent) -> cosmic::app::Task<Message> {
        match event {
            ConnectionEvent::AgentFound(agent) => {
                let id = agent.info.id.clone();
                tracing::info!(agent = %id, name = %agent.info.name, "agent discovered");
                self.connection_states
                    .entry(id)
                    .or_insert(ConnectionState::Discovered);
                // Insert or update in discovered list.
                if let Some(existing) = self
                    .discovered_agents
                    .iter_mut()
                    .find(|a| a.info.id == agent.info.id)
                {
                    *existing = agent;
                } else {
                    self.discovered_agents.push(agent);
                }
            }

            ConnectionEvent::AgentUpdated(agent) => {
                if let Some(existing) = self
                    .discovered_agents
                    .iter_mut()
                    .find(|a| a.info.id == agent.info.id)
                {
                    *existing = agent;
                }
            }

            ConnectionEvent::AgentLost(id) => {
                tracing::info!(agent = %id, "agent lost");
                self.discovered_agents.retain(|a| a.info.id != id);
                self.connection_states.remove(&id);
                self.telemetry_hub.remove_agent(&id);
                self.console_log_hub.remove_agent(&id);
                // If the lost agent was connected, clear connection.
                if self.connected_agent_id() == Some(&id) {
                    self.connected_agent = None;
                    if self.nav_page.requires_connection() {
                        self.nav_page = NavPage::Devices;
                    }
                }
            }

            ConnectionEvent::StateChanged { agent_id, state } => {
                self.connection_states.insert(agent_id, state);
            }

            ConnectionEvent::PairingNeeded {
                agent_id,
                code,
                expires_in,
            } => {
                tracing::info!(agent = %agent_id, "pairing required");
                self.pairing_dialog = Some(PairingDialog::new(agent_id, code, expires_in));
            }

            ConnectionEvent::AgentEvent {
                agent_id,
                msg_type,
                message,
            } => {
                self.route_agent_event(&agent_id, msg_type, &message);
            }
        }
        cosmic::app::Task::none()
    }

    /// Routes agent events to the appropriate subsystem.
    fn route_agent_event(
        &mut self,
        agent_id: &str,
        msg_type: MessageType,
        message: &capydeploy_protocol::envelope::Message,
    ) {
        match msg_type {
            MessageType::TelemetryData => {
                if let Ok(Some(data)) =
                    message.parse_payload::<capydeploy_protocol::telemetry::TelemetryData>()
                {
                    self.telemetry_hub.process_data(agent_id, &data);
                    self.update_telemetry_widgets(agent_id);
                }
            }
            MessageType::TelemetryStatus => {
                if let Ok(Some(event)) =
                    message.parse_payload::<capydeploy_protocol::telemetry::TelemetryStatusEvent>()
                {
                    self.telemetry_hub.process_status(agent_id, &event);
                }
            }
            MessageType::ConsoleLogData => {
                if let Ok(Some(batch)) =
                    message.parse_payload::<capydeploy_protocol::console_log::ConsoleLogBatch>()
                {
                    self.console_log_hub.process_batch(agent_id, &batch);
                }
            }
            MessageType::ConsoleLogStatus => {
                if let Ok(Some(event)) = message
                    .parse_payload::<capydeploy_protocol::console_log::ConsoleLogStatusEvent>()
                {
                    self.console_log_hub.process_status(agent_id, &event);
                }
            }
            _ => {
                // Deploy events, etc. handled in later steps.
            }
        }
    }

    /// Updates canvas widget values from the latest telemetry data.
    fn update_telemetry_widgets(&mut self, agent_id: &str) {
        let Some(agent) = self.telemetry_hub.get_agent(agent_id) else {
            return;
        };

        // Update gauges from latest snapshot.
        if let Some(latest) = agent.latest() {
            if let Some(cpu) = &latest.cpu {
                self.telemetry_widgets.cpu_gauge.set_value(cpu.usage_percent);
                self.telemetry_widgets
                    .cpu_temp_gauge
                    .set_value(cpu.temp_celsius);
            }
            if let Some(gpu) = &latest.gpu {
                self.telemetry_widgets.gpu_gauge.set_value(gpu.usage_percent);
            }
            if let Some(mem) = &latest.memory {
                self.telemetry_widgets.mem_gauge.set_value(mem.usage_percent);
            }
        }

        // Update sparklines from history ring buffers.
        let cpu_data: Vec<f64> = agent.cpu_usage_history().iter().copied().collect();
        self.telemetry_widgets.cpu_sparkline.set_data(&cpu_data);

        let gpu_data: Vec<f64> = agent.gpu_usage_history().iter().copied().collect();
        self.telemetry_widgets.gpu_sparkline.set_data(&gpu_data);

        let mem_data: Vec<f64> = agent.mem_usage_history().iter().copied().collect();
        self.telemetry_widgets.mem_sparkline.set_data(&mem_data);
    }
}

// ---------------------------------------------------------------------------
// View helpers
// ---------------------------------------------------------------------------

impl Hub {
    fn view_sidebar(&self) -> Element<'_, Message> {
        let mut nav = widget::column().spacing(4).padding([16, 8]);

        // Brand header.
        let header = widget::column()
            .push(widget::text::title4("CapyDeploy").class(theme::CYAN))
            .push(widget::text::caption(&self.config.name).class(theme::MUTED_TEXT))
            .spacing(2)
            .padding([0, 8, 16, 8]);
        nav = nav.push(header);

        // Nav items.
        let is_connected = self.is_connected();
        for page in NavPage::ALL {
            let is_active = self.nav_page == page;
            let disabled = page.requires_connection() && !is_connected;

            let btn = if disabled {
                let label = widget::text(page.label()).class(theme::MUTED_TEXT);
                widget::button::custom(label)
            } else if is_active {
                let label = widget::text(page.label()).class(theme::CYAN);
                widget::button::custom(label).on_press(Message::NavigateTo(page))
            } else {
                let label = widget::text(page.label());
                widget::button::custom(label).on_press(Message::NavigateTo(page))
            };

            let btn_element: Element<'_, Message> = btn.width(Length::Fill).into();

            if is_active {
                nav = nav.push(
                    container(btn_element)
                        .class(cosmic::theme::Container::Custom(Box::new(
                            theme::nav_active_bg,
                        ))),
                );
            } else {
                nav = nav.push(btn_element);
            }
        }

        // Connection status at bottom.
        nav = nav.push(widget::Space::with_height(Length::Fill));

        let (status_text, status_color) = if let Some(agent) = &self.connected_agent {
            (agent.agent.info.name.as_str(), theme::CONNECTED_COLOR)
        } else {
            ("No agent connected", theme::MUTED_TEXT)
        };
        nav = nav.push(
            widget::text::caption(status_text)
                .class(status_color)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        );

        container(nav)
            .width(Length::Fixed(200.0))
            .height(Length::Fill)
            .class(cosmic::theme::Container::Custom(Box::new(
                theme::sidebar_bg,
            )))
            .into()
    }

    fn view_content(&self) -> Element<'_, Message> {
        let content = match self.nav_page {
            NavPage::Devices => crate::views::devices::view(
                &self.discovered_agents,
                self.connected_agent_id(),
                &self.connection_states,
            ),
            NavPage::Deploy => self.view_placeholder("Deploy"),
            NavPage::Games => self.view_placeholder("Games"),
            NavPage::Telemetry => crate::views::telemetry::view(
                &self.telemetry_hub,
                self.connected_agent_id(),
                &self.telemetry_widgets,
            ),
            NavPage::Console => crate::views::console::view(
                &self.console_log_hub,
                self.connected_agent_id(),
                self.console_level_filter,
                &self.console_search,
            ),
            NavPage::Settings => self.view_placeholder("Settings"),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
            .class(cosmic::theme::Container::Custom(Box::new(content_bg)))
            .into()
    }

    fn view_placeholder(&self, name: &'static str) -> Element<'_, Message> {
        widget::column()
            .push(widget::text::title3(name))
            .push(widget::text("Coming soon...").class(theme::MUTED_TEXT))
            .spacing(8)
            .into()
    }
}

/// Content area background.
fn content_bg(_theme: &cosmic::Theme) -> iced_container::Style {
    iced_container::Style {
        background: Some(cosmic::iced::Background::Color(theme::DARK_BG)),
        ..Default::default()
    }
}
