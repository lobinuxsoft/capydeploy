//! CapyDeploy desktop agent entry point.

mod app;
mod config;

use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    // Initialize structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "starting CapyDeploy agent"
    );

    // Load configuration.
    let config = config::Config::load()?;
    tracing::info!(name = %config.name, "configuration loaded");

    // Build and run the tokio runtime.
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(app::run(config))?;

    tracing::info!("agent shut down cleanly");
    Ok(())
}
