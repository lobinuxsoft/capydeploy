use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::Value;

use capydeploy_protocol::telemetry::TelemetryStatusEvent;

use crate::buffer::RingBuffer;

/// Default ring buffer capacity: 300 samples (5 min at 1 s interval).
const DEFAULT_CAPACITY: usize = 300;

/// Data older than this is considered stale (no active telemetry stream).
const STALE_THRESHOLD: Duration = Duration::from_secs(5);

/// Per-agent telemetry state with dynamic metric histories.
///
/// Instead of tracking a fixed set of metrics, this walks incoming JSON and
/// automatically creates a [`RingBuffer`] for every numeric value found under
/// nested objects. Paths use dot notation (e.g. `"cpu.usagePercent"`).
/// Top-level numeric fields like `timestamp` are excluded.
#[derive(Debug, Clone)]
pub struct AgentTelemetry {
    latest: Option<Value>,
    last_received: Option<Instant>,
    enabled: bool,
    interval: i32,
    metrics: HashMap<String, RingBuffer<f64>>,
    capacity: usize,
}

impl AgentTelemetry {
    /// Create a new per-agent state with the given history capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            latest: None,
            last_received: None,
            enabled: false,
            interval: 0,
            metrics: HashMap::new(),
            capacity,
        }
    }

    /// Ingest a telemetry data snapshot.
    ///
    /// Walks the JSON object and pushes every numeric value found inside nested
    /// objects into a ring buffer keyed by `"section.field"`. Top-level numbers
    /// (like `timestamp`) are skipped — only nested numeric fields are tracked.
    pub fn process_data(&mut self, data: &Value) {
        if let Value::Object(top) = data {
            for (section, value) in top {
                if let Value::Object(fields) = value {
                    for (field, val) in fields {
                        if let Some(n) = val.as_f64() {
                            let key = format!("{section}.{field}");
                            self.metrics
                                .entry(key)
                                .or_insert_with(|| RingBuffer::new(self.capacity))
                                .push(n);
                        }
                    }
                }
            }
        }
        self.latest = Some(data.clone());
        self.last_received = Some(Instant::now());
    }

    /// Update telemetry enabled/interval from a status event.
    pub fn process_status(&mut self, event: &TelemetryStatusEvent) {
        self.enabled = event.enabled;
        self.interval = event.interval;
    }

    /// The most recent full telemetry snapshot, if any.
    pub fn latest(&self) -> Option<&Value> {
        self.latest.as_ref()
    }

    /// Whether the agent has telemetry enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Whether telemetry data is stale (no data received recently).
    pub fn is_stale(&self) -> bool {
        self.last_received
            .is_some_and(|t| t.elapsed() > STALE_THRESHOLD)
    }

    /// Telemetry reporting interval in seconds.
    pub fn interval(&self) -> i32 {
        self.interval
    }

    /// Get the ring buffer for a specific metric path (e.g. `"cpu.usagePercent"`).
    pub fn history(&self, path: &str) -> Option<&RingBuffer<f64>> {
        self.metrics.get(path)
    }

    /// List all tracked metric paths, sorted alphabetically.
    pub fn metric_keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.metrics.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys
    }
}

/// Top-level telemetry state manager for all connected agents.
///
/// The Hub app feeds [`ConnectionEvent::AgentEvent`] data into this struct,
/// and the iced UI reads from it. All methods are synchronous.
#[derive(Debug, Clone)]
pub struct TelemetryHub {
    agents: HashMap<String, AgentTelemetry>,
    capacity: usize,
}

impl TelemetryHub {
    /// Create a hub with the default buffer capacity (300 samples).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a hub with a custom per-metric buffer capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            agents: HashMap::new(),
            capacity,
        }
    }

    /// Route a telemetry data snapshot to the appropriate agent state.
    pub fn process_data(&mut self, agent_id: &str, data: &Value) {
        self.agents
            .entry(agent_id.to_owned())
            .or_insert_with(|| AgentTelemetry::new(self.capacity))
            .process_data(data);
    }

    /// Route a telemetry status event to the appropriate agent state.
    pub fn process_status(&mut self, agent_id: &str, event: &TelemetryStatusEvent) {
        self.agents
            .entry(agent_id.to_owned())
            .or_insert_with(|| AgentTelemetry::new(self.capacity))
            .process_status(event);
    }

    /// Look up the telemetry state for a specific agent.
    pub fn get_agent(&self, agent_id: &str) -> Option<&AgentTelemetry> {
        self.agents.get(agent_id)
    }

    /// Remove an agent's telemetry state (e.g. on disconnect).
    pub fn remove_agent(&mut self, agent_id: &str) {
        self.agents.remove(agent_id);
    }

    /// List all tracked agent IDs.
    pub fn agent_ids(&self) -> Vec<&str> {
        self.agents.keys().map(String::as_str).collect()
    }

    /// Reset all state.
    pub fn clear(&mut self) {
        self.agents.clear();
    }
}

impl Default for TelemetryHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn sample_data_full() -> Value {
        json!({
            "timestamp": 1700000000,
            "cpu": { "usagePercent": 45.0, "tempCelsius": 65.0, "freqMHz": 3200.0 },
            "gpu": { "usagePercent": 80.0, "tempCelsius": 75.0, "freqMHz": 1800.0 },
            "memory": { "totalBytes": 16000000000_i64, "availableBytes": 8000000000_i64, "usagePercent": 50.0 }
        })
    }

    fn sample_data_cpu_only() -> Value {
        json!({
            "timestamp": 1700000001,
            "cpu": { "usagePercent": 30.0, "tempCelsius": 55.0, "freqMHz": 2800.0 }
        })
    }

    fn sample_data_partial_cpu() -> Value {
        json!({
            "timestamp": 1700000002,
            "cpu": { "usagePercent": 25.0, "freqMHz": 1963.0 }
        })
    }

    // --- AgentTelemetry tests ---

    #[test]
    fn process_data_full_metrics() {
        let mut agent = AgentTelemetry::new(10);
        let data = sample_data_full();

        agent.process_data(&data);

        assert_eq!(agent.latest().unwrap()["timestamp"], 1700000000);
        assert_eq!(agent.history("cpu.usagePercent").unwrap().len(), 1);
        assert_eq!(agent.history("cpu.tempCelsius").unwrap().len(), 1);
        assert_eq!(agent.history("gpu.usagePercent").unwrap().len(), 1);
        assert_eq!(agent.history("gpu.tempCelsius").unwrap().len(), 1);
        assert_eq!(agent.history("memory.usagePercent").unwrap().len(), 1);

        assert_eq!(
            agent.history("cpu.usagePercent").unwrap().last(),
            Some(&45.0)
        );
        assert_eq!(
            agent.history("gpu.tempCelsius").unwrap().last(),
            Some(&75.0)
        );
        assert_eq!(
            agent.history("memory.usagePercent").unwrap().last(),
            Some(&50.0)
        );
    }

    #[test]
    fn process_data_tracks_all_numeric_fields() {
        let mut agent = AgentTelemetry::new(10);
        agent.process_data(&sample_data_full());

        // All numeric fields from cpu, gpu, memory should be tracked
        let keys = agent.metric_keys();
        assert!(keys.contains(&"cpu.usagePercent"));
        assert!(keys.contains(&"cpu.tempCelsius"));
        assert!(keys.contains(&"cpu.freqMHz"));
        assert!(keys.contains(&"gpu.usagePercent"));
        assert!(keys.contains(&"gpu.tempCelsius"));
        assert!(keys.contains(&"gpu.freqMHz"));
        assert!(keys.contains(&"memory.totalBytes"));
        assert!(keys.contains(&"memory.availableBytes"));
        assert!(keys.contains(&"memory.usagePercent"));

        // Top-level "timestamp" should NOT be tracked
        assert!(agent.history("timestamp").is_none());
    }

    #[test]
    fn process_data_partial_metrics() {
        let mut agent = AgentTelemetry::new(10);
        agent.process_data(&sample_data_cpu_only());

        assert_eq!(agent.history("cpu.usagePercent").unwrap().len(), 1);
        assert_eq!(agent.history("cpu.tempCelsius").unwrap().len(), 1);
        assert!(agent.history("gpu.usagePercent").is_none());
        assert!(agent.history("memory.usagePercent").is_none());
    }

    #[test]
    fn process_data_partial_cpu_missing_temp() {
        let mut agent = AgentTelemetry::new(10);
        agent.process_data(&sample_data_partial_cpu());

        // usagePercent pushed, but tempCelsius was absent — not tracked
        assert_eq!(agent.history("cpu.usagePercent").unwrap().len(), 1);
        assert_eq!(
            agent.history("cpu.usagePercent").unwrap().last(),
            Some(&25.0)
        );
        assert!(agent.history("cpu.tempCelsius").is_none());
    }

    #[test]
    fn process_data_unknown_sections() {
        let mut agent = AgentTelemetry::new(10);
        let data = json!({
            "timestamp": 1700000000,
            "battery": { "capacity": 85.0 },
            "power": { "tdpWatts": 15.0, "powerWatts": 12.3 },
            "fan": { "rpm": 2100.0 }
        });
        agent.process_data(&data);

        assert_eq!(
            agent.history("battery.capacity").unwrap().last(),
            Some(&85.0)
        );
        assert_eq!(agent.history("power.tdpWatts").unwrap().last(), Some(&15.0));
        assert_eq!(
            agent.history("power.powerWatts").unwrap().last(),
            Some(&12.3)
        );
        assert_eq!(agent.history("fan.rpm").unwrap().last(), Some(&2100.0));
    }

    #[test]
    fn process_status_updates_fields() {
        let mut agent = AgentTelemetry::new(10);
        assert!(!agent.enabled());
        assert_eq!(agent.interval(), 0);

        agent.process_status(&TelemetryStatusEvent {
            enabled: true,
            interval: 5,
        });

        assert!(agent.enabled());
        assert_eq!(agent.interval(), 5);
    }

    #[test]
    fn history_accumulates() {
        let mut agent = AgentTelemetry::new(10);

        for i in 0..5 {
            let data = json!({
                "timestamp": 1700000000 + i,
                "cpu": {
                    "usagePercent": i as f64 * 10.0,
                    "tempCelsius": 50.0 + i as f64,
                    "freqMHz": 3000.0
                }
            });
            agent.process_data(&data);
        }

        let history = agent.history("cpu.usagePercent").unwrap();
        assert_eq!(history.len(), 5);
        let values: Vec<&f64> = history.iter().collect();
        assert_eq!(values, vec![&0.0, &10.0, &20.0, &30.0, &40.0]);
    }

    // --- TelemetryHub tests ---

    #[test]
    fn multiple_agents_tracked_independently() {
        let mut hub = TelemetryHub::with_capacity(10);

        hub.process_data("agent-1", &sample_data_full());
        hub.process_data("agent-2", &sample_data_cpu_only());

        let a1 = hub.get_agent("agent-1").unwrap();
        assert_eq!(a1.history("gpu.usagePercent").unwrap().len(), 1);

        let a2 = hub.get_agent("agent-2").unwrap();
        assert!(a2.history("gpu.usagePercent").is_none());
    }

    #[test]
    fn remove_agent_cleans_up() {
        let mut hub = TelemetryHub::with_capacity(10);
        hub.process_data("agent-1", &sample_data_full());
        hub.process_data("agent-2", &sample_data_full());

        hub.remove_agent("agent-1");

        assert!(hub.get_agent("agent-1").is_none());
        assert!(hub.get_agent("agent-2").is_some());
    }

    #[test]
    fn get_unknown_agent_returns_none() {
        let hub = TelemetryHub::new();
        assert!(hub.get_agent("nonexistent").is_none());
    }

    #[test]
    fn clear_removes_all() {
        let mut hub = TelemetryHub::with_capacity(10);
        hub.process_data("agent-1", &sample_data_full());
        hub.process_data("agent-2", &sample_data_full());

        hub.clear();

        assert!(hub.agent_ids().is_empty());
    }

    #[test]
    fn agent_ids_returns_tracked() {
        let mut hub = TelemetryHub::with_capacity(10);
        hub.process_data("alpha", &sample_data_full());
        hub.process_data("beta", &sample_data_full());

        let mut ids = hub.agent_ids();
        ids.sort();
        assert_eq!(ids, vec!["alpha", "beta"]);
    }

    #[test]
    fn process_status_creates_agent_if_absent() {
        let mut hub = TelemetryHub::with_capacity(10);

        hub.process_status(
            "new-agent",
            &TelemetryStatusEvent {
                enabled: true,
                interval: 3,
            },
        );

        let agent = hub.get_agent("new-agent").unwrap();
        assert!(agent.enabled());
        assert_eq!(agent.interval(), 3);
        assert!(agent.latest().is_none());
    }
}
