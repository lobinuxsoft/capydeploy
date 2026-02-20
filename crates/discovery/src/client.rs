use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use capydeploy_protocol::AgentInfo;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use tokio::sync::mpsc;

use crate::DiscoveryError;
use crate::types::{DEFAULT_TTL, DiscoveredAgent, DiscoveryEvent, EventType, SERVICE_NAME};

/// Discovers agents on the local network via mDNS/DNS-SD.
pub struct Client {
    agents: Arc<RwLock<HashMap<String, DiscoveredAgent>>>,
    events_tx: mpsc::Sender<DiscoveryEvent>,
    events_rx: Option<mpsc::Receiver<DiscoveryEvent>>,
    timeout: Duration,
}

impl Client {
    /// Creates a new mDNS discovery client.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(16);
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            events_tx: tx,
            events_rx: Some(rx),
            timeout: Duration::from_secs(DEFAULT_TTL),
        }
    }

    /// Sets the stale agent timeout.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Takes the event receiver. Can only be called once.
    pub fn take_events(&mut self) -> Option<mpsc::Receiver<DiscoveryEvent>> {
        self.events_rx.take()
    }

    /// Performs a one-time mDNS query and returns discovered agents.
    pub async fn discover(
        &self,
        timeout: Duration,
    ) -> Result<Vec<DiscoveredAgent>, DiscoveryError> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| DiscoveryError::Mdns(format!("failed to create mDNS daemon: {e}")))?;

        let service_type = format!("{SERVICE_NAME}.local.");
        let receiver = daemon
            .browse(&service_type)
            .map_err(|e| DiscoveryError::Mdns(format!("failed to browse mDNS: {e}")))?;

        let mut agents = Vec::new();
        let deadline = Instant::now() + timeout;

        while Instant::now() < deadline {
            match tokio::time::timeout(
                deadline.saturating_duration_since(Instant::now()),
                tokio::task::spawn_blocking({
                    let receiver = receiver.clone();
                    move || receiver.recv_timeout(Duration::from_millis(100))
                }),
            )
            .await
            {
                Ok(Ok(Ok(event))) => {
                    if let Some(agent) = self.process_event(&event) {
                        agents.push(agent);
                    }
                }
                _ => {
                    // Timeout or error — continue until deadline
                    if Instant::now() >= deadline {
                        break;
                    }
                }
            }
        }

        let _ = daemon.shutdown();
        Ok(agents)
    }

    /// Begins continuous agent discovery.
    ///
    /// Creates a single `ServiceDaemon` that browses for the entire
    /// session, avoiding the repeated create/destroy cycle that causes
    /// noisy shutdown errors in the `mdns_sd` crate.
    pub async fn start_continuous_discovery(
        &self,
        mut cancel: tokio::sync::watch::Receiver<bool>,
        prune_interval: Duration,
    ) {
        let daemon = match ServiceDaemon::new() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("discovery: failed to create mDNS daemon: {e}");
                return;
            }
        };

        let service_type = format!("{SERVICE_NAME}.local.");
        let event_rx = match daemon.browse(&service_type) {
            Ok(rx) => rx,
            Err(e) => {
                eprintln!("discovery: failed to browse mDNS: {e}");
                let _ = daemon.shutdown();
                return;
            }
        };

        let mut prune_ticker = tokio::time::interval(prune_interval);
        prune_ticker.tick().await; // consume first immediate tick

        loop {
            tokio::select! {
                // Receive mDNS events via spawn_blocking (recv is blocking).
                result = tokio::task::spawn_blocking({
                    let rx = event_rx.clone();
                    move || rx.recv_timeout(Duration::from_millis(500))
                }) => {
                    if let Ok(Ok(event)) = result {
                        self.process_event(&event);
                    }
                }
                // Periodically prune stale agents.
                _ = prune_ticker.tick() => {
                    self.prune_stale_agents().await;
                }
                // Cancellation signal.
                _ = cancel.changed() => {
                    let _ = daemon.shutdown();
                    return;
                }
            }
        }
    }

    /// Processes an mDNS service event into a `DiscoveredAgent`.
    fn process_event(&self, event: &ServiceEvent) -> Option<DiscoveredAgent> {
        let ServiceEvent::ServiceResolved(info) = event else {
            return None;
        };

        let mut agent_info = AgentInfo {
            id: String::new(),
            name: String::new(),
            platform: String::new(),
            version: String::new(),
            accept_connections: false,
            supported_image_formats: vec![],
        };

        // Parse TXT records
        for property in info.get_properties().iter() {
            let key = property.key();
            let val = property.val_str();
            match key {
                "id" => agent_info.id = val.to_string(),
                "name" => agent_info.name = val.to_string(),
                "platform" => agent_info.platform = val.to_string(),
                "version" => agent_info.version = val.to_string(),
                _ => {}
            }
        }

        // Use fullname as ID if not in TXT
        if agent_info.id.is_empty() {
            agent_info.id = info.get_fullname().to_string();
        }
        if agent_info.name.is_empty() {
            agent_info.name = info.get_hostname().to_string();
        }

        // Collect IPs (filter out loopback and link-local)
        let ips: Vec<IpAddr> = info
            .get_addresses()
            .iter()
            .filter_map(|ip| {
                let ip4 = match ip {
                    std::net::IpAddr::V4(v4) => *v4,
                    _ => return None,
                };
                if ip4.octets()[0] == 127 {
                    return None;
                }
                if ip4.octets()[0] == 169 && ip4.octets()[1] == 254 {
                    return None;
                }
                Some(IpAddr::V4(ip4))
            })
            .collect();

        let now = Instant::now();
        let agent = DiscoveredAgent {
            info: agent_info.clone(),
            host: info.get_hostname().to_string(),
            port: info.get_port(),
            ips,
            discovered_at: Some(now),
            last_seen: Some(now),
        };

        // Update or add agent
        let mut agents = self.agents.write().unwrap();
        let event_type = if let Some(existing) = agents.get_mut(&agent_info.id) {
            existing.last_seen = Some(now);
            existing.ips = agent.ips.clone();
            existing.port = agent.port;
            EventType::Updated
        } else {
            agents.insert(agent_info.id.clone(), agent.clone());
            EventType::Discovered
        };
        drop(agents);

        let _ = self.events_tx.try_send(DiscoveryEvent {
            event_type,
            agent: agent.clone(),
        });

        Some(agent)
    }

    /// Removes agents that haven't been seen recently.
    async fn prune_stale_agents(&self) {
        let mut agents = self.agents.write().unwrap();
        let stale: Vec<(String, DiscoveredAgent)> = agents
            .iter()
            .filter(|(_, agent)| agent.is_stale(self.timeout))
            .map(|(id, agent)| (id.clone(), agent.clone()))
            .collect();

        for (id, agent) in stale {
            agents.remove(&id);
            let _ = self.events_tx.try_send(DiscoveryEvent {
                event_type: EventType::Lost,
                agent,
            });
        }
    }

    /// Returns all currently known agents.
    pub fn get_agents(&self) -> Vec<DiscoveredAgent> {
        self.agents.read().unwrap().values().cloned().collect()
    }

    /// Returns a specific agent by ID.
    pub fn get_agent(&self, id: &str) -> Option<DiscoveredAgent> {
        self.agents.read().unwrap().get(id).cloned()
    }

    /// Removes an agent from tracking.
    pub fn remove_agent(&self, id: &str) {
        let mut agents = self.agents.write().unwrap();
        if let Some(agent) = agents.remove(id) {
            drop(agents);
            let _ = self.events_tx.try_send(DiscoveryEvent {
                event_type: EventType::Lost,
                agent,
            });
        }
    }

    /// Removes all tracked agents.
    pub fn clear(&self) {
        self.agents.write().unwrap().clear();
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(id: &str) -> DiscoveredAgent {
        DiscoveredAgent {
            info: AgentInfo {
                id: id.into(),
                name: format!("Agent {id}"),
                platform: "linux".into(),
                version: "0.1.0".into(),
                accept_connections: true,
                supported_image_formats: vec![],
            },
            host: "test.local".into(),
            port: 8765,
            ips: vec!["192.168.1.10".parse().unwrap()],
            discovered_at: Some(Instant::now()),
            last_seen: Some(Instant::now()),
        }
    }

    #[test]
    fn client_agent_tracking() {
        let client = Client::new();

        // Manually insert an agent
        {
            let mut agents = client.agents.write().unwrap();
            agents.insert("a1".into(), make_agent("a1"));
        }

        assert_eq!(client.get_agents().len(), 1);
        assert!(client.get_agent("a1").is_some());
        assert!(client.get_agent("missing").is_none());
    }

    #[test]
    fn client_remove_agent() {
        let client = Client::new();

        {
            let mut agents = client.agents.write().unwrap();
            agents.insert("a1".into(), make_agent("a1"));
        }

        client.remove_agent("a1");
        assert!(client.get_agents().is_empty());
    }

    #[test]
    fn client_clear() {
        let client = Client::new();

        {
            let mut agents = client.agents.write().unwrap();
            agents.insert("a1".into(), make_agent("a1"));
            agents.insert("a2".into(), make_agent("a2"));
        }

        client.clear();
        assert!(client.get_agents().is_empty());
    }
}
