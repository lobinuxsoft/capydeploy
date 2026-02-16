pub mod client;
pub mod platform;
pub mod server;
pub mod types;

// Re-export primary types.
pub use client::Client;
pub use platform::detect_platform;
pub use server::{Server, get_hostname, get_local_ips};
pub use types::{
    DEFAULT_TTL, DiscoveredAgent, DiscoveryEvent, EventType, SERVICE_NAME, ServiceInfo,
};

/// Errors for discovery operations.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("mDNS error: {0}")]
    Mdns(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("configuration error: {0}")]
    Config(String),
}
