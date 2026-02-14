//! Brand colors and container style overrides.

use cosmic::iced::widget::container as iced_container;
use cosmic::iced::{Background, Border, Color};

/// CapyDeploy brand cyan.
pub const CYAN: Color = Color::from_rgb(0.30, 0.70, 1.0);

/// CapyDeploy brand orange.
#[allow(dead_code)] // Used in deploy progress and toast styling.
pub const ORANGE: Color = Color::from_rgb(1.0, 0.60, 0.20);

/// Dark background for the main content area.
pub const DARK_BG: Color = Color::from_rgb(0.10, 0.10, 0.12);

/// Sidebar background — slightly lighter than main.
pub const SIDEBAR_BG: Color = Color::from_rgb(0.08, 0.08, 0.10);

/// Muted text color.
pub const MUTED_TEXT: Color = Color::from_rgb(0.50, 0.50, 0.55);

/// Active/selected nav item highlight.
pub const NAV_ACTIVE: Color = Color::from_rgba(0.30, 0.70, 1.0, 0.15);

/// Static dark background for canvas containers — avoids Z-fighting that
/// `Container::Card` causes (Card has hover effects that trigger redraws).
#[allow(dead_code)] // Used by telemetry and deploy views.
pub fn canvas_bg(_theme: &cosmic::Theme) -> iced_container::Style {
    iced_container::Style {
        background: Some(Background::Color(DARK_BG)),
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Sidebar container style.
pub fn sidebar_bg(_theme: &cosmic::Theme) -> iced_container::Style {
    iced_container::Style {
        background: Some(Background::Color(SIDEBAR_BG)),
        border: Border {
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Active nav item background.
pub fn nav_active_bg(_theme: &cosmic::Theme) -> iced_container::Style {
    iced_container::Style {
        background: Some(Background::Color(NAV_ACTIVE)),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
