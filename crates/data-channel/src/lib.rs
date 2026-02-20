//! TCP data channel for bulk file transfers.
//!
//! Provides a dedicated TCP connection for streaming file data between
//! Hub and Agent, bypassing WebSocket framing overhead. The WS connection
//! remains for signaling (init, progress, completion).
//!
//! # Wire format
//!
//! See [`wire`] module for the binary protocol specification.

pub mod client;
pub mod error;
pub mod server;
pub mod token;
pub mod wire;

pub use error::DataChannelError;
pub use token::{generate_token, validate_token};
pub use wire::FileHeader;

use std::time::Duration;

/// TCP read/write buffer size (256 KB).
pub const TCP_BUFFER_SIZE: usize = 256 * 1024;

/// Timeout for the TCP connection attempt.
pub const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for the authentication handshake.
pub const TCP_AUTH_TIMEOUT: Duration = Duration::from_secs(5);

/// Capability string advertised by agents that support TCP data channel.
pub const CAPABILITY_TCP_DATA_CHANNEL: &str = "tcp_data_channel";
