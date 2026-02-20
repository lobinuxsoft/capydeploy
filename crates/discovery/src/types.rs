use std::fmt;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use capydeploy_protocol::AgentInfo;
use serde::{Deserialize, Serialize};

/// mDNS service type for CapyDeploy agents.
pub const SERVICE_NAME: &str = "_capydeploy._tcp";

/// Default TTL for mDNS records (seconds).
pub const DEFAULT_TTL: u64 = 120;

/// An agent discovered via mDNS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredAgent {
    pub info: AgentInfo,
    pub host: String,
    pub port: u16,
    #[serde(skip)]
    pub ips: Vec<IpAddr>,
    #[serde(skip)]
    pub discovered_at: Option<Instant>,
    #[serde(skip)]
    pub last_seen: Option<Instant>,
}

impl DiscoveredAgent {
    /// Returns the address (IP:port or host:port) for connecting to the agent.
    pub fn address(&self) -> String {
        if let Some(ip) = self.ips.first() {
            format!("{ip}:{}", self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// Returns the WebSocket address for the agent.
    pub fn websocket_address(&self) -> String {
        format!("ws://{}/ws", self.address())
    }

    /// Returns true if the agent hasn't been seen recently.
    pub fn is_stale(&self, timeout: Duration) -> bool {
        match self.last_seen {
            Some(last) => last.elapsed() > timeout,
            None => true,
        }
    }
}

/// Information for advertising an agent via mDNS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub version: String,
    pub port: u16,
    #[serde(skip)]
    pub ips: Vec<IpAddr>,
}

impl ServiceInfo {
    /// Converts to a protocol `AgentInfo`.
    pub fn to_agent_info(&self) -> AgentInfo {
        AgentInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            platform: self.platform.clone(),
            version: self.version.clone(),
            accept_connections: false,
            supported_image_formats: vec![],
        }
    }
}

/// A discovery or loss event.
#[derive(Debug, Clone)]
pub struct DiscoveryEvent {
    pub event_type: EventType,
    pub agent: DiscoveredAgent,
}

/// Type of discovery event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Discovered,
    Updated,
    Lost,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::Discovered => write!(f, "discovered"),
            EventType::Updated => write!(f, "updated"),
            EventType::Lost => write!(f, "lost"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_agent() -> DiscoveredAgent {
        DiscoveredAgent {
            info: AgentInfo {
                id: "test-id".into(),
                name: "Test".into(),
                platform: "linux".into(),
                version: "0.1.0".into(),
                accept_connections: true,
                supported_image_formats: vec![],
            },
            host: "test.local".into(),
            port: 8765,
            ips: vec!["192.168.1.100".parse().unwrap()],
            discovered_at: Some(Instant::now()),
            last_seen: Some(Instant::now()),
        }
    }

    #[test]
    fn address_with_ip() {
        let agent = test_agent();
        assert_eq!(agent.address(), "192.168.1.100:8765");
    }

    #[test]
    fn address_without_ip() {
        let mut agent = test_agent();
        agent.ips.clear();
        assert_eq!(agent.address(), "test.local:8765");
    }

    #[test]
    fn websocket_address() {
        let agent = test_agent();
        assert_eq!(agent.websocket_address(), "ws://192.168.1.100:8765/ws");
    }

    #[test]
    fn is_stale_fresh() {
        let agent = test_agent();
        assert!(!agent.is_stale(Duration::from_secs(120)));
    }

    #[test]
    fn is_stale_no_last_seen() {
        let mut agent = test_agent();
        agent.last_seen = None;
        assert!(agent.is_stale(Duration::from_secs(1)));
    }

    #[test]
    fn service_info_to_agent_info() {
        let si = ServiceInfo {
            id: "s1".into(),
            name: "Server".into(),
            platform: "steamdeck".into(),
            version: "0.6.0".into(),
            port: 9999,
            ips: vec![],
        };
        let ai = si.to_agent_info();
        assert_eq!(ai.id, "s1");
        assert_eq!(ai.platform, "steamdeck");
    }

    #[test]
    fn event_type_display() {
        assert_eq!(EventType::Discovered.to_string(), "discovered");
        assert_eq!(EventType::Updated.to_string(), "updated");
        assert_eq!(EventType::Lost.to_string(), "lost");
    }
}
