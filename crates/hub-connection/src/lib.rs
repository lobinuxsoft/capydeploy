//! Hub connection manager for connecting to Agents.
//!
//! Provides WebSocket client, mDNS discovery integration,
//! and Hub-Agent pairing flow.

pub mod manager;
pub mod pairing;
pub mod ws_client;

pub use manager::{
    ConnectedAgent, ConnectionEvent, ConnectionManager, ConnectionState, HubIdentity,
    ReconnectConfig,
};
pub use pairing::TokenStore;
pub use ws_client::{HandshakeResult, WsClient, WsError};
