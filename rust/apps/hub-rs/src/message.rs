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
    /// No-op — used when a background task is cancelled.
    Noop,

    // -- Navigation --
    /// Switch to a sidebar page.
    NavigateTo(NavPage),

    // -- Connection lifecycle --
    /// mDNS discovery started and subscription is active.
    DiscoveryStarted,
    /// User clicked refresh discovery.
    RefreshDiscovery,
    /// A connection event arrived from the subscription.
    ConnectionEvent(ConnectionEvent),
    /// User clicked connect on an agent.
    ConnectAgent(String),
    /// Connection attempt finished.
    ConnectResult(Result<ConnectedAgent, String>),
    /// User clicked disconnect.
    DisconnectAgent,
    /// User clicked cancel on an active reconnect.
    CancelReconnect(String),
    /// Connected agent restored after a successful reconnect.
    ReconnectRestored(Option<ConnectedAgent>),

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
    /// Toggle visibility of the SteamGridDB API key field.
    ToggleApiKeyVisibility,
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
    // ConsoleSetEnabled removed — Decky agent auto-starts console log.

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
    /// Batch of thumbnails fetched from remote URLs.
    ArtworkThumbnailsBatch(Vec<(String, Vec<u8>)>),
    /// User wants to pick a local artwork file.
    ArtworkSelectLocalFile,
    /// Local file picker returned a path.
    ArtworkLocalFileResult(Option<String>),
    /// Toggle a filter option (style, dimension, MIME, animation type).
    ArtworkToggleFilter(ArtworkFilterField, String),
    /// Toggle NSFW filter on/off.
    ArtworkToggleNsfw,
    /// Toggle Humor filter on/off.
    ArtworkToggleHumor,
    /// Reset all filters to defaults.
    ArtworkResetFilters,
    /// Toggle the filter panel visibility.
    ArtworkShowFilters,
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
    LaunchOptions,
    Tags,
}

/// Filter fields in the artwork selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtworkFilterField {
    Style,
    Dimension,
    MimeType,
    ImageType,
}

/// Fields in settings that can be edited.
#[derive(Debug, Clone)]
pub enum SettingField {
    Name,
    SteamGridDbApiKey,
    GameLogDir,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_page_all_contains_every_variant() {
        assert_eq!(NavPage::ALL.len(), 6);
        assert_eq!(NavPage::ALL[0], NavPage::Devices);
        assert_eq!(NavPage::ALL[1], NavPage::Deploy);
        assert_eq!(NavPage::ALL[2], NavPage::Games);
        assert_eq!(NavPage::ALL[3], NavPage::Telemetry);
        assert_eq!(NavPage::ALL[4], NavPage::Console);
        assert_eq!(NavPage::ALL[5], NavPage::Settings);
    }

    #[test]
    fn nav_page_labels() {
        assert_eq!(NavPage::Devices.label(), "Devices");
        assert_eq!(NavPage::Deploy.label(), "Deploy");
        assert_eq!(NavPage::Games.label(), "Games");
        assert_eq!(NavPage::Telemetry.label(), "Telemetry");
        assert_eq!(NavPage::Console.label(), "Console");
        assert_eq!(NavPage::Settings.label(), "Settings");
    }

    #[test]
    fn nav_page_requires_connection() {
        // Pages that require a connected agent.
        assert!(NavPage::Deploy.requires_connection());
        assert!(NavPage::Games.requires_connection());
        assert!(NavPage::Telemetry.requires_connection());
        assert!(NavPage::Console.requires_connection());

        // Pages that work without connection.
        assert!(!NavPage::Devices.requires_connection());
        assert!(!NavPage::Settings.requires_connection());
    }
}
