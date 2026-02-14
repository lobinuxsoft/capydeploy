//! Hub message types for the iced/cosmic runtime.

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
#[allow(dead_code)] // Variants added incrementally across steps.
pub enum Message {
    // -- Navigation --
    /// Switch to a sidebar page.
    NavigateTo(NavPage),

    // -- System --
    /// Periodic tick for animations and timer-based events.
    Tick,
}
