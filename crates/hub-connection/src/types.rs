//! Public types for the hub connection manager.

use std::time::Duration;

use capydeploy_discovery::types::DiscoveredAgent;
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages::AgentStatusResponse;

/// Connection state for an Agent.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Agent discovered via mDNS but not connected.
    Discovered,
    /// WebSocket handshake in progress.
    Connecting,
    /// Connected and authenticated.
    Connected,
    /// Agent requires pairing before connection.
    PairingRequired,
    /// Connection lost, attempting to reconnect.
    Reconnecting { attempt: u32 },
    /// Connection lost.
    Disconnected,
}

/// An Agent that has been connected.
#[derive(Debug, Clone)]
pub struct ConnectedAgent {
    pub agent: DiscoveredAgent,
    pub status: AgentStatusResponse,
}

/// Events emitted by the connection manager.
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// An Agent was discovered via mDNS.
    AgentFound(DiscoveredAgent),
    /// A discovered Agent's info was updated.
    AgentUpdated(DiscoveredAgent),
    /// A discovered Agent was lost (mDNS timeout).
    AgentLost(String),
    /// Connection state changed for an Agent.
    StateChanged {
        agent_id: String,
        state: ConnectionState,
    },
    /// Pairing is required. The UI should prompt the user for the code.
    PairingNeeded {
        agent_id: String,
        code: String,
        expires_in: i32,
    },
    /// A push event was received from a connected Agent.
    AgentEvent {
        agent_id: String,
        msg_type: MessageType,
        message: Message,
    },
    /// Reconnection is in progress for an Agent.
    Reconnecting {
        agent_id: String,
        attempt: u32,
        next_retry_secs: f64,
    },
    /// Agent's protocol version is deprecated (still works, but outdated).
    ProtocolWarning { agent_id: String, message: String },
}

/// Configuration for automatic reconnection with exponential backoff.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Initial delay before the first reconnection attempt.
    pub initial_delay: Duration,
    /// Maximum delay between attempts (backoff cap).
    pub max_delay: Duration,
    /// Multiplier for each subsequent attempt.
    pub backoff_factor: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(15),
            backoff_factor: 2.0,
        }
    }
}

impl ReconnectConfig {
    /// Calculates the delay for a given attempt number (1-based),
    /// with ±25% jitter to avoid thundering herd.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp = attempt.saturating_sub(1).min(63) as i32;
        let secs = self.initial_delay.as_secs_f64() * self.backoff_factor.powi(exp);
        let capped = secs.min(self.max_delay.as_secs_f64());
        // Add ±25% jitter.
        let jitter = capped * 0.25;
        let offset = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as f64
            / u32::MAX as f64)
            * 2.0
            - 1.0; // [-1.0, 1.0)
        let with_jitter = (capped + jitter * offset).max(0.05);
        Duration::from_secs_f64(with_jitter)
    }
}

/// Hub identity used in connection handshakes.
#[derive(Debug, Clone)]
pub struct HubIdentity {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub hub_id: String,
}

/// Maximum reconnect attempts without mDNS visibility before giving up.
pub(crate) const MAX_NO_MDNS_ATTEMPTS: u32 = 30;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_state_equality() {
        assert_eq!(ConnectionState::Discovered, ConnectionState::Discovered);
        assert_ne!(ConnectionState::Connected, ConnectionState::Connecting);
        assert_eq!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 1 },
        );
        assert_ne!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 2 },
        );
    }

    #[test]
    fn hub_identity_clone() {
        let hub = HubIdentity {
            name: "Test Hub".into(),
            version: "0.1.0".into(),
            platform: "linux".into(),
            hub_id: "hub-test-1".into(),
        };
        let hub2 = hub.clone();
        assert_eq!(hub.name, hub2.name);
        assert_eq!(hub.hub_id, hub2.hub_id);
    }

    #[test]
    fn reconnect_config_defaults() {
        let config = ReconnectConfig::default();
        assert_eq!(config.initial_delay, Duration::from_millis(250));
        assert_eq!(config.max_delay, Duration::from_secs(15));
        assert!((config.backoff_factor - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reconnect_config_delay_backoff() {
        let config = ReconnectConfig::default();
        // Base delays: 250ms, 500ms, 1s, 2s, 4s, 8s, 15s (capped), 15s...
        // With ±25% jitter, check that values are within expected range.
        let expected_base = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 15.0, 15.0];
        for (i, &base) in expected_base.iter().enumerate() {
            let delay = config.delay_for_attempt((i + 1) as u32);
            let secs = delay.as_secs_f64();
            let lo = base * 0.74; // -26% to allow for jitter rounding
            let hi = base * 1.26; // +26%
            assert!(
                secs >= lo && secs <= hi,
                "attempt {}: {secs:.3}s not in [{lo:.3}, {hi:.3}]",
                i + 1
            );
        }
    }
}
