//! CapyDeploy Hub entry point.

mod app;
mod bridge;
mod config;
mod dialogs;
mod message;
mod subscriptions;
mod theme;
mod views;

use tracing_subscriber::EnvFilter;

fn main() -> cosmic::iced::Result {
    // Initialize structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "starting CapyDeploy Hub"
    );

    // Load configuration.
    let hub_config = match config::HubConfig::load() {
        Ok(c) => {
            tracing::info!(name = %c.name, "configuration loaded");
            c
        }
        Err(e) => {
            tracing::warn!(error = %e, "failed to load config, using defaults");
            config::HubConfig::default()
        }
    };

    let settings = cosmic::app::Settings::default()
        .size(cosmic::iced::Size::new(1100.0, 750.0));

    cosmic::app::run::<app::Hub>(settings, hub_config)
}
