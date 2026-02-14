//! Hub message types for the iced/cosmic runtime.

use cosmic::widget::ToastId;

use capydeploy_hub_connection::{ConnectedAgent, ConnectionEvent};
use capydeploy_hub_deploy::{DeployEvent, DeployResult};
use capydeploy_hub_games::InstalledGame;
use capydeploy_steamgriddb::types::{ImageData, SearchResult};

use crate::dialogs::artwork::ArtworkTab;

/// Navigation pages in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPage {
    Devices,
    Deploy,
    Games,
    Telemetry,
    Console,
    Settings,
}

impl NavPage {
    /// Display label for the sidebar.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Devices => "Devices",
            Self::Deploy => "Deploy",
            Self::Games => "Games",
            Self::Telemetry => "Telemetry",
            Self::Console => "Console",
            Self::Settings => "Settings",
        }
    }

    /// Whether this page requires an active agent connection.
    pub fn requires_connection(&self) -> bool {
        matches!(
            self,
            Self::Deploy | Self::Games | Self::Telemetry | Self::Console
        )
    }

    /// All pages in sidebar order.
    pub const ALL: [NavPage; 6] = [
        Self::Devices,
        Self::Deploy,
        Self::Games,
        Self::Telemetry,
        Self::Console,
        Self::Settings,
    ];
}

/// Top-level message enum for the Hub application.
#[derive(Debug, Clone)]
pub enum Message {
    // -- Navigation --
    /// Switch to a sidebar page.
    NavigateTo(NavPage),

    // -- Connection lifecycle --
    /// mDNS discovery started and subscription is active.
    DiscoveryStarted,
    /// A connection event arrived from the subscription.
    ConnectionEvent(ConnectionEvent),
    /// User clicked connect on an agent.
    ConnectAgent(String),
    /// Connection attempt finished.
    ConnectResult(Result<ConnectedAgent, String>),
    /// User clicked disconnect.
    DisconnectAgent,

    // -- Pairing --
    /// User is typing the pairing code.
    PairingCodeInput(String),
    /// User confirmed the pairing code.
    ConfirmPairing,
    /// User cancelled the pairing dialog.
    CancelPairing,
    /// Pairing attempt finished.
    PairingResult(Result<ConnectedAgent, String>),

    // -- Deploy --
    /// Create a new empty game setup for editing.
    NewSetup,
    /// Start editing an existing setup by ID.
    EditSetup(String),
    /// Save the currently edited setup.
    SaveSetup,
    /// Cancel editing without saving.
    CancelEditSetup,
    /// Delete a game setup by ID.
    DeleteSetup(String),
    /// Update a field in the currently editing setup.
    UpdateSetupField(SetupField, String),
    /// Open a native folder picker for the setup's local path.
    BrowseLocalPath,
    /// Folder picker result for local path.
    BrowseLocalPathResult(Option<String>),
    /// Start deploying a game setup to the connected agent.
    StartDeploy(String),
    /// Deploy completed (one result per agent).
    DeployComplete(Vec<DeployResult>),
    /// Real-time deploy progress event from the orchestrator.
    DeployProgress(DeployEvent),
    /// Dismiss the deploy status message.
    DismissDeployStatus,

    // -- Games --
    /// Fetch installed games from the connected agent.
    RefreshGames,
    /// Games list fetched successfully.
    GamesLoaded(Result<Vec<InstalledGame>, String>),
    /// Delete a game from the agent by AppID.
    DeleteGame(u32),
    /// Game deletion result.
    DeleteGameResult(Result<u32, String>),
    /// Open artwork editor for an installed game.
    EditGameArtwork(u32),
    /// Save artwork edits for an installed game (triggered from ArtworkSave).
    SaveGameArtwork,
    /// Result of updating game artwork on the agent.
    SaveGameArtworkResult(Result<u32, String>),

    // -- Settings --
    /// Update a setting field value.
    UpdateSetting(SettingField, String),
    /// Save settings to disk.
    SaveSettings,
    /// Open a native folder picker for the game log directory.
    BrowseGameLogDir,
    /// Folder picker result for game log directory.
    BrowseGameLogDirResult(Option<String>),
    /// Clear the game log directory setting.
    ClearGameLogDir,

    // -- Console Log --
    /// Toggle a log level bit in the UI filter.
    ConsoleToggleLevel(u32),
    /// Update the console search text.
    ConsoleSearchInput(String),
    /// Clear the console log buffer.
    ConsoleClear,
    /// Change the source filter for console log entries.
    ConsoleSourceFilter(String),
    /// Enable or disable console log streaming on the agent.
    ConsoleSetEnabled(bool),
    /// Result of a console log enable/disable request.
    ConsoleSetEnabledResult(Result<bool, String>),

    // -- Artwork Selector --
    /// Open the artwork selector for the currently editing setup.
    OpenArtworkSelector,
    /// Search input changed.
    ArtworkSearchInput(String),
    /// User submitted the search (Enter or button).
    ArtworkSearchSubmit,
    /// Search results arrived.
    ArtworkSearchResults(Result<Vec<SearchResult>, String>),
    /// User selected a game from search results.
    ArtworkSelectGame(i32, String),
    /// User switched artwork tab.
    ArtworkTabChanged(ArtworkTab),
    /// Images loaded for a tab.
    ArtworkImagesLoaded(ArtworkTab, Result<Vec<ImageData>, String>),
    /// User selected an image URL for a tab.
    ArtworkSelectImage(ArtworkTab, String),
    /// Load more images for the current artwork tab (next page).
    ArtworkLoadMore,
    /// Clear all artwork selections.
    ArtworkClearAll,
    /// Save artwork selections to the editing setup.
    ArtworkSave,
    /// Close the artwork selector without saving.
    ArtworkCancel,

    // -- Toasts --
    /// Auto-dismiss or manual close of a toast notification.
    CloseToast(ToastId),

    // -- System --
    /// Periodic tick for animations and timer-based events.
    #[allow(dead_code)]
    Tick,
}

/// Fields in a game setup that can be edited.
#[derive(Debug, Clone)]
pub enum SetupField {
    Name,
    LocalPath,
    Executable,
    InstallPath,
    LaunchOptions,
    Tags,
}

/// Fields in settings that can be edited.
#[derive(Debug, Clone)]
pub enum SettingField {
    Name,
    SteamGridDbApiKey,
    GameLogDir,
}
