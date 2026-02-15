//! Brand colors and container style overrides.
//!
//! Palette based on the original Svelte Hub frontend:
//! - Background: slate-900 `#0f172a`
//! - Surface/cards: slate-800 `#1e293b`
//! - Cyan accent: `#06b6d4` (tailwind cyan-500)
//! - Orange branding: `#f97316` (tailwind orange-500)
//! - Muted text: slate-400 `#94a3b8`
//! - Borders: `rgba(71, 85, 105, 0.5)` (slate-600 @ 50%)

use cosmic::iced::widget::container as iced_container;
use cosmic::iced::{Background, Border, Color};

/// CapyDeploy brand cyan — `#06b6d4`.
pub const CYAN: Color = Color::from_rgb(0.024, 0.714, 0.831);

/// CapyDeploy brand orange — `#f97316`.
#[allow(dead_code)] // Used in deploy progress and toast styling.
pub const ORANGE: Color = Color::from_rgb(0.976, 0.451, 0.086);

/// Dark background for the main content area — slate-900 `#0f172a`.
pub const DARK_BG: Color = Color::from_rgb(0.059, 0.090, 0.165);

/// Sidebar background — slightly darker than main.
pub const SIDEBAR_BG: Color = Color::from_rgb(0.043, 0.067, 0.133);

/// Card / surface background — slate-800 `#1e293b`.
pub const SURFACE_BG: Color = Color::from_rgb(0.118, 0.161, 0.231);

/// Muted text color — slate-400 `#94a3b8`.
pub const MUTED_TEXT: Color = Color::from_rgb(0.580, 0.639, 0.722);

/// Active/selected nav item highlight.
pub const NAV_ACTIVE: Color = Color::from_rgba(0.024, 0.714, 0.831, 0.15);

/// Connected agent badge color — emerald-500 `#10b981`.
pub const CONNECTED_COLOR: Color = Color::from_rgb(0.063, 0.725, 0.506);

/// Subtle border color — slate-600 `#475569` @ 50%.
const BORDER_COLOR: Color = Color::from_rgba(0.278, 0.333, 0.412, 0.5);

/// Card container style — used for all content cards across views.
#[allow(dead_code)] // Used by telemetry, deploy, devices, settings, games, console.
pub fn canvas_bg(_theme: &cosmic::Theme) -> iced_container::Style {
    iced_container::Style {
        background: Some(Background::Color(SURFACE_BG)),
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 8.0.into(),
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
