//! Hub connection manager for connecting to Agents.
//!
//! Provides WebSocket client, mDNS discovery integration,
//! and Hub-Agent pairing flow.

mod discovery;
pub mod manager;
pub mod pairing;
pub(crate) mod reconnection;
pub mod types;
pub mod ws_client;

pub use manager::ConnectionManager;
pub use pairing::TokenStore;
pub use types::{ConnectedAgent, ConnectionEvent, ConnectionState, HubIdentity, ReconnectConfig};
pub use ws_client::{HandshakeResult, WsClient, WsError};
