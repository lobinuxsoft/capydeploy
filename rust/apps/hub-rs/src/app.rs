//! Hub application — `cosmic::Application` implementation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cosmic::app::Core;
use cosmic::iced::widget::container as iced_container;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, container, Toast, Toasts};
use cosmic::{Application, Element};

use capydeploy_discovery::types::DiscoveredAgent;
use capydeploy_hub_connection::pairing::default_token_path;
use capydeploy_hub_connection::{
    ConnectedAgent, ConnectionEvent, ConnectionManager, ConnectionState, HubIdentity, TokenStore,
};
use capydeploy_hub_console_log::ConsoleLogHub;
use capydeploy_hub_telemetry::TelemetryHub;
use capydeploy_protocol::constants::{self, MessageType};

use capydeploy_hub_deploy::{
    build_artwork_assignment, DeployConfig, DeployEvent, DeployOrchestrator, GameSetup,
};
use capydeploy_hub_games::{GamesManager, InstalledGame};

use crate::config::HubConfig;
use crate::dialogs::artwork::{ArtworkDialog, ArtworkTab};
use crate::dialogs::pairing::PairingDialog;
use crate::message::{Message, NavPage, SettingField, SetupField};
use crate::theme;
use crate::views::deploy::DeployStatus;
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
    console_source_filter: String,
    console_search: String,

    // Deploy state.
    editing_setup: Option<GameSetup>,
    deploy_status: Option<DeployStatus>,
    deploy_events_rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<DeployEvent>>>>,

    // Games state.
    installed_games: Vec<InstalledGame>,
    games_loading: bool,

    // Settings state.
    settings_dirty: bool,
    api_key_hidden: bool,

    // Artwork selector dialog.
    artwork_dialog: Option<ArtworkDialog>,
    /// When editing artwork for an installed game, holds the AppID.
    game_artwork_app_id: Option<u32>,

    // Toast notifications.
    toasts: Toasts<Message>,
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

    /// Pushes a toast and wraps the resulting task for the cosmic runtime.
    fn push_toast(&mut self, text: impl Into<String>) -> cosmic::app::Task<Message> {
        self.toasts
            .push(Toast::new(text))
            .map(cosmic::action::app)
    }

    /// Loads artwork images for a tab from SteamGridDB.
    fn load_artwork_tab(
        &mut self,
        tab: ArtworkTab,
        game_id: i32,
        page: i32,
    ) -> cosmic::app::Task<Message> {
        if let Some(dialog) = &mut self.artwork_dialog {
            dialog.loading.insert(tab, true);
        }
        let api_key = self.config.steamgriddb_api_key.clone();
        cosmic::app::Task::perform(
            async move {
                let client = capydeploy_steamgriddb::Client::new(&api_key)?;
                match tab {
                    ArtworkTab::Capsule => {
                        let grids = client.get_grids(game_id, None, page).await?;
                        // Filter to portrait (height > width).
                        Ok(grids
                            .into_iter()
                            .filter(|g| g.height > g.width)
                            .collect())
                    }
                    ArtworkTab::Wide => {
                        let grids = client.get_grids(game_id, None, page).await?;
                        // Filter to landscape (width > height).
                        Ok(grids
                            .into_iter()
                            .filter(|g| g.width > g.height)
                            .collect())
                    }
                    ArtworkTab::Hero => client.get_heroes(game_id, None, page).await,
                    ArtworkTab::Logo => client.get_logos(game_id, None, page).await,
                    ArtworkTab::Icon => client.get_icons(game_id, None, page).await,
                }
            },
            move |result| {
                cosmic::action::app(Message::ArtworkImagesLoaded(
                    tab,
                    result.map_err(|e| e.to_string()),
                ))
            },
        )
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
            console_source_filter: String::new(),
            console_search: String::new(),
            editing_setup: None,
            deploy_status: None,
            deploy_events_rx: Arc::new(Mutex::new(None)),
            installed_games: Vec::new(),
            games_loading: false,
            settings_dirty: false,
            api_key_hidden: true,
            artwork_dialog: None,
            game_artwork_app_id: None,
            toasts: Toasts::new(Message::CloseToast),
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
        let mut subs =
            vec![crate::subscriptions::connection_events(self.connection_mgr.clone())];

        // Stream deploy progress events while a deploy is active.
        if matches!(self.deploy_status, Some(DeployStatus::Deploying { .. })) {
            subs.push(crate::subscriptions::deploy_events(
                self.deploy_events_rx.clone(),
            ));
        }

        cosmic::iced::Subscription::batch(subs)
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

            Message::RefreshDiscovery => {
                self.discovered_agents.clear();
                self.connection_states
                    .retain(|_, s| matches!(s, ConnectionState::Connected));
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
                    let name = agent.agent.info.name.clone();
                    self.on_connected(agent);
                    return self.push_toast(format!("Connected to {name}"));
                }
                Err(e) => {
                    // Pairing-required errors are handled via ConnectionEvent::PairingNeeded.
                    if !e.contains("pairing required") {
                        tracing::warn!(error = %e, "connection failed");
                        return self.push_toast(format!("Connection failed: {e}"));
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
                    let name = agent.agent.info.name.clone();
                    self.on_connected(agent);
                    return self.push_toast(format!("Paired with {name}"));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "pairing failed");
                    if let Some(dialog) = &mut self.pairing_dialog {
                        dialog.confirming = false;
                        dialog.input.clear();
                    }
                    return self.push_toast(format!("Pairing failed: {e}"));
                }
            },

            // -- Games --
            Message::RefreshGames => {
                if let Some(agent_id) = self.connected_agent_id().map(String::from) {
                    self.games_loading = true;
                    let mgr = self.connection_mgr.clone();
                    let games_mgr = GamesManager::new(reqwest::Client::new());
                    return cosmic::app::Task::perform(
                        async move {
                            let bridge =
                                crate::bridge::ConnectionBridge::new(mgr, agent_id);
                            games_mgr.get_installed_games(&bridge).await
                        },
                        |result| {
                            cosmic::action::app(Message::GamesLoaded(
                                result.map_err(|e| e.to_string()),
                            ))
                        },
                    );
                }
            }

            Message::GamesLoaded(result) => {
                self.games_loading = false;
                match result {
                    Ok(games) => {
                        let count = games.len();
                        self.installed_games = games;
                        return self.push_toast(format!("{count} games loaded"));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to fetch installed games");
                        return self.push_toast(format!("Failed to load games: {e}"));
                    }
                }
            }

            Message::DeleteGame(app_id) => {
                if let Some(agent_id) = self.connected_agent_id().map(String::from) {
                    let mgr = self.connection_mgr.clone();
                    let games_mgr = GamesManager::new(reqwest::Client::new());
                    return cosmic::app::Task::perform(
                        async move {
                            let bridge =
                                crate::bridge::ConnectionBridge::new(mgr, agent_id);
                            games_mgr
                                .delete_game(&bridge, app_id)
                                .await
                                .map(|_| app_id)
                        },
                        |result| {
                            cosmic::action::app(Message::DeleteGameResult(
                                result.map_err(|e| e.to_string()),
                            ))
                        },
                    );
                }
            }

            Message::DeleteGameResult(result) => match result {
                Ok(app_id) => {
                    tracing::info!(app_id, "game deleted");
                    self.installed_games.retain(|g| g.app_id != app_id);
                    return self.push_toast(format!("Game {app_id} deleted"));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to delete game");
                    return self.push_toast(format!("Delete failed: {e}"));
                }
            },

            Message::EditGameArtwork(app_id) => {
                let game_name = self
                    .installed_games
                    .iter()
                    .find(|g| g.app_id == app_id)
                    .map(|g| g.name.as_str())
                    .unwrap_or("Unknown");
                self.game_artwork_app_id = Some(app_id);
                self.artwork_dialog = Some(ArtworkDialog::new(
                    game_name, 0, "", "", "", "", "",
                ));
            }

            Message::SaveGameArtwork => {
                if let Some(app_id) = self.game_artwork_app_id.take()
                    && let Some(dialog) = self.artwork_dialog.take()
                {
                    let sel = dialog.selection();
                    let artwork = capydeploy_hub_games::ArtworkUpdate {
                        grid: sel.grid_portrait,
                        banner: sel.grid_landscape,
                        hero: sel.hero_image,
                        logo: sel.logo_image,
                        icon: sel.icon_image,
                    };

                    if let Some(agent_id) = self.connected_agent_id().map(String::from) {
                        let mgr = self.connection_mgr.clone();
                        let games_mgr = GamesManager::new(reqwest::Client::new());
                        return cosmic::app::Task::perform(
                            async move {
                                let bridge =
                                    crate::bridge::ConnectionBridge::new(mgr, agent_id);
                                games_mgr
                                    .update_game_artwork(&bridge, app_id, &artwork)
                                    .await
                                    .map(|_| app_id)
                            },
                            |result| {
                                cosmic::action::app(Message::SaveGameArtworkResult(
                                    result.map_err(|e| e.to_string()),
                                ))
                            },
                        );
                    }
                }
            }

            Message::SaveGameArtworkResult(result) => match result {
                Ok(app_id) => {
                    tracing::info!(app_id, "game artwork updated");
                    return self.push_toast(format!("Artwork updated for game {app_id}"));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to update game artwork");
                    return self.push_toast(format!("Artwork update failed: {e}"));
                }
            },

            // -- Settings --
            Message::UpdateSetting(field, value) => {
                self.settings_dirty = true;
                match field {
                    SettingField::Name => self.config.name = value,
                    SettingField::SteamGridDbApiKey => self.config.steamgriddb_api_key = value,
                    SettingField::GameLogDir => self.config.game_log_dir = value,
                }
            }

            Message::ToggleApiKeyVisibility => {
                self.api_key_hidden = !self.api_key_hidden;
            }

            Message::SaveSettings => {
                match self.config.save() {
                    Ok(()) => {
                        self.settings_dirty = false;
                        tracing::info!("settings saved");
                        return self.push_toast("Settings saved");
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to save settings");
                        return self.push_toast(format!("Save failed: {e}"));
                    }
                }
            }

            Message::BrowseGameLogDir => {
                return cosmic::app::Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Select Game Log Directory")
                            .pick_folder()
                            .await
                            .map(|h| h.path().to_string_lossy().into_owned())
                    },
                    |result| cosmic::action::app(Message::BrowseGameLogDirResult(result)),
                );
            }

            Message::BrowseGameLogDirResult(Some(path)) => {
                self.config.game_log_dir = path;
                self.settings_dirty = true;
            }

            Message::BrowseGameLogDirResult(None) => {}

            Message::ClearGameLogDir => {
                self.config.game_log_dir.clear();
                self.settings_dirty = true;
            }

            // -- Console Log --
            Message::ConsoleToggleLevel(bit) => {
                self.console_level_filter ^= bit;
            }

            Message::ConsoleSourceFilter(source) => {
                self.console_source_filter = source;
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
                    let msg = if enabled {
                        "Console log enabled"
                    } else {
                        "Console log disabled"
                    };
                    tracing::info!(enabled, "console log streaming toggled");
                    return self.push_toast(msg);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to toggle console log");
                    return self.push_toast(format!("Console toggle failed: {e}"));
                }
            },

            // -- Deploy --
            Message::NewSetup => {
                self.editing_setup = Some(GameSetup {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: String::new(),
                    local_path: String::new(),
                    executable: String::new(),
                    launch_options: String::new(),
                    tags: String::new(),
                    install_path: String::new(),
                    griddb_game_id: 0,
                    grid_portrait: String::new(),
                    grid_landscape: String::new(),
                    hero_image: String::new(),
                    logo_image: String::new(),
                    icon_image: String::new(),
                });
            }

            Message::EditSetup(id) => {
                if let Some(setup) = self.config.game_setups.iter().find(|s| s.id == id) {
                    self.editing_setup = Some(setup.clone());
                }
            }

            Message::SaveSetup => {
                if let Some(setup) = self.editing_setup.take() {
                    if let Some(existing) = self
                        .config
                        .game_setups
                        .iter_mut()
                        .find(|s| s.id == setup.id)
                    {
                        *existing = setup;
                    } else {
                        self.config.game_setups.push(setup);
                    }
                    if let Err(e) = self.config.save() {
                        tracing::warn!(error = %e, "failed to save config");
                    }
                }
            }

            Message::CancelEditSetup => {
                self.editing_setup = None;
            }

            Message::DeleteSetup(id) => {
                self.config.game_setups.retain(|s| s.id != id);
                if let Err(e) = self.config.save() {
                    tracing::warn!(error = %e, "failed to save config");
                }
            }

            Message::UpdateSetupField(field, value) => {
                if let Some(setup) = &mut self.editing_setup {
                    match field {
                        SetupField::Name => setup.name = value,
                        SetupField::LocalPath => setup.local_path = value,
                        SetupField::Executable => setup.executable = value,
                        SetupField::InstallPath => setup.install_path = value,
                        SetupField::LaunchOptions => setup.launch_options = value,
                        SetupField::Tags => setup.tags = value,
                    }
                }
            }

            Message::BrowseLocalPath => {
                return cosmic::app::Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Select Game Folder")
                            .pick_folder()
                            .await
                            .map(|h| h.path().to_string_lossy().into_owned())
                    },
                    |result| cosmic::action::app(Message::BrowseLocalPathResult(result)),
                );
            }

            Message::BrowseLocalPathResult(Some(path)) => {
                if let Some(setup) = &mut self.editing_setup {
                    setup.local_path = path;
                }
            }

            Message::BrowseLocalPathResult(None) => {}

            Message::StartDeploy(setup_id) => {
                if let Some(agent_id) = self.connected_agent_id().map(String::from)
                    && let Some(setup) = self
                        .config
                        .game_setups
                        .iter()
                        .find(|s| s.id == setup_id)
                        .cloned()
                {
                    let setup_name = setup.name.clone();
                    self.deploy_status = Some(DeployStatus::Deploying {
                        setup_name,
                        progress: 0.0,
                        status_msg: "Starting...".into(),
                    });

                    let artwork = build_artwork_assignment(&setup);
                    let config = DeployConfig { setup, artwork };
                    let mgr = self.connection_mgr.clone();

                    // Create orchestrator and store events receiver for the subscription.
                    let mut orch = DeployOrchestrator::new();
                    if let Some(events_rx) = orch.take_events() {
                        *self.deploy_events_rx.lock().unwrap() = Some(events_rx);
                    }

                    return cosmic::app::Task::perform(
                        async move {
                            let bridge =
                                crate::bridge::ConnectionBridge::new(mgr, agent_id);
                            orch.deploy(config, vec![&bridge]).await
                        },
                        |results| cosmic::action::app(Message::DeployComplete(results)),
                    );
                }
            }

            Message::DeployProgress(event) => {
                if let DeployEvent::Progress {
                    progress, status, ..
                } = event
                    && let Some(DeployStatus::Deploying {
                        progress: p,
                        status_msg,
                        ..
                    }) = &mut self.deploy_status
                {
                    *p = progress;
                    *status_msg = status;
                }
            }

            Message::DeployComplete(results) => {
                // Clear the deploy events receiver.
                *self.deploy_events_rx.lock().unwrap() = None;

                if let Some(result) = results.into_iter().next() {
                    let setup_name = self
                        .deploy_status
                        .as_ref()
                        .map(|s| match s {
                            DeployStatus::Deploying { setup_name, .. } => setup_name.clone(),
                            DeployStatus::Success { setup_name, .. } => setup_name.clone(),
                            DeployStatus::Failed { setup_name, .. } => setup_name.clone(),
                        })
                        .unwrap_or_default();

                    if result.success {
                        let app_id = result.app_id.unwrap_or(0);
                        self.deploy_status = Some(DeployStatus::Success {
                            setup_name: setup_name.clone(),
                            app_id,
                        });
                        return self.push_toast(format!(
                            "{setup_name} deployed (AppID: {app_id})"
                        ));
                    } else {
                        let error =
                            result.error.unwrap_or_else(|| "unknown error".into());
                        self.deploy_status = Some(DeployStatus::Failed {
                            setup_name: setup_name.clone(),
                            error: error.clone(),
                        });
                        return self.push_toast(format!(
                            "Deploy failed: {error}"
                        ));
                    }
                }
            }

            Message::DismissDeployStatus => {
                self.deploy_status = None;
            }

            // -- Artwork Selector --
            Message::OpenArtworkSelector => {
                if let Some(setup) = &self.editing_setup {
                    self.artwork_dialog = Some(ArtworkDialog::new(
                        &setup.name,
                        setup.griddb_game_id,
                        &setup.grid_portrait,
                        &setup.grid_landscape,
                        &setup.hero_image,
                        &setup.logo_image,
                        &setup.icon_image,
                    ));
                }
            }

            Message::ArtworkSearchInput(text) => {
                if let Some(dialog) = &mut self.artwork_dialog {
                    dialog.search_query = text;
                }
            }

            Message::ArtworkSearchSubmit => {
                if let Some(dialog) = &mut self.artwork_dialog {
                    if dialog.search_query.is_empty() {
                        return cosmic::app::Task::none();
                    }
                    dialog.searching = true;
                    let api_key = self.config.steamgriddb_api_key.clone();
                    let query = dialog.search_query.clone();
                    return cosmic::app::Task::perform(
                        async move {
                            let client = capydeploy_steamgriddb::Client::new(&api_key)?;
                            client.search(&query).await
                        },
                        |result| {
                            cosmic::action::app(Message::ArtworkSearchResults(
                                result.map_err(|e| e.to_string()),
                            ))
                        },
                    );
                }
            }

            Message::ArtworkSearchResults(result) => {
                if let Some(dialog) = &mut self.artwork_dialog {
                    dialog.searching = false;
                    match result {
                        Ok(results) => dialog.search_results = results,
                        Err(e) => {
                            tracing::warn!(error = %e, "artwork search failed");
                            return self.push_toast(format!("Search failed: {e}"));
                        }
                    }
                }
            }

            Message::ArtworkSelectGame(game_id, game_name) => {
                if let Some(dialog) = &mut self.artwork_dialog {
                    dialog.selected_game_id = Some(game_id);
                    dialog.selected_game_name = game_name;
                    dialog.griddb_game_id = game_id;
                    dialog.images.clear();
                    dialog.pages.clear();
                    // Load first page for default tab.
                    return self.load_artwork_tab(ArtworkTab::Capsule, game_id, 0);
                }
            }

            Message::ArtworkTabChanged(tab) => {
                if let Some(dialog) = &mut self.artwork_dialog {
                    dialog.active_tab = tab;
                    // Load images if not already loaded.
                    if !dialog.images.contains_key(&tab)
                        && let Some(game_id) = dialog.selected_game_id
                    {
                        return self.load_artwork_tab(tab, game_id, 0);
                    }
                }
            }

            Message::ArtworkImagesLoaded(tab, result) => {
                if let Some(dialog) = &mut self.artwork_dialog {
                    dialog.loading.insert(tab, false);
                    match result {
                        Ok(new_images) => {
                            // Append to existing images (pagination support).
                            dialog
                                .images
                                .entry(tab)
                                .or_default()
                                .extend(new_images);
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, tab = ?tab, "artwork load failed");
                            dialog.images.entry(tab).or_default();
                            return self.push_toast(format!("Load failed: {e}"));
                        }
                    }
                }
            }

            Message::ArtworkSelectImage(tab, url) => {
                if let Some(dialog) = &mut self.artwork_dialog {
                    dialog.select_image(tab, &url);
                }
            }

            Message::ArtworkLoadMore => {
                if let Some(dialog) = &mut self.artwork_dialog
                    && let Some(game_id) = dialog.selected_game_id
                {
                    let tab = dialog.active_tab;
                    let next_page = dialog.pages.get(&tab).copied().unwrap_or(0) + 1;
                    dialog.pages.insert(tab, next_page);
                    return self.load_artwork_tab(tab, game_id, next_page);
                }
            }

            Message::ArtworkClearAll => {
                if let Some(dialog) = &mut self.artwork_dialog {
                    dialog.grid_portrait.clear();
                    dialog.grid_landscape.clear();
                    dialog.hero_image.clear();
                    dialog.logo_image.clear();
                    dialog.icon_image.clear();
                    dialog.griddb_game_id = 0;
                }
            }

            Message::ArtworkSave => {
                // Dispatch based on context: game setup vs installed game.
                if self.game_artwork_app_id.is_some() {
                    return self.update(Message::SaveGameArtwork);
                }
                if let Some(dialog) = self.artwork_dialog.take() {
                    let sel = dialog.selection();
                    if let Some(setup) = &mut self.editing_setup {
                        setup.griddb_game_id = sel.griddb_game_id;
                        setup.grid_portrait = sel.grid_portrait;
                        setup.grid_landscape = sel.grid_landscape;
                        setup.hero_image = sel.hero_image;
                        setup.logo_image = sel.logo_image;
                        setup.icon_image = sel.icon_image;
                    }
                    return self.push_toast("Artwork selection saved");
                }
            }

            Message::ArtworkCancel => {
                self.artwork_dialog = None;
                self.game_artwork_app_id = None;
            }

            // -- Toasts --
            Message::CloseToast(id) => {
                self.toasts.remove(id);
            }

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
        let main = if let Some(dialog) = &self.pairing_dialog {
            let overlay = crate::dialogs::pairing::view(dialog);
            cosmic::widget::popover(main)
                .modal(true)
                .popup(overlay)
                .on_close(Message::CancelPairing)
                .into()
        } else if let Some(dialog) = &self.artwork_dialog {
            crate::dialogs::artwork::view(dialog)
        } else {
            main
        };

        // Toast notification overlay.
        cosmic::widget::toaster(&self.toasts, main)
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
            NavPage::Deploy => {
                let deploying = matches!(
                    self.deploy_status,
                    Some(DeployStatus::Deploying { .. })
                );
                crate::views::deploy::view(
                    &self.config.game_setups,
                    self.editing_setup.as_ref(),
                    self.deploy_status.as_ref(),
                    self.is_connected(),
                    deploying,
                )
            }
            NavPage::Games => crate::views::games::view(
                &self.installed_games,
                self.games_loading,
            ),
            NavPage::Telemetry => crate::views::telemetry::view(
                &self.telemetry_hub,
                self.connected_agent_id(),
                &self.telemetry_widgets,
            ),
            NavPage::Console => crate::views::console::view(
                &self.console_log_hub,
                self.connected_agent_id(),
                self.console_level_filter,
                &self.console_source_filter,
                &self.console_search,
            ),
            NavPage::Settings => crate::views::settings::view(
                &self.config,
                self.settings_dirty,
                self.api_key_hidden,
            ),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
            .class(cosmic::theme::Container::Custom(Box::new(content_bg)))
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
