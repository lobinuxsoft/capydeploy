use std::collections::HashMap;

use capydeploy_protocol::telemetry::{TelemetryData, TelemetryStatusEvent};

use crate::buffer::RingBuffer;

/// Default ring buffer capacity: 300 samples (5 min at 1 s interval).
const DEFAULT_CAPACITY: usize = 300;

/// Per-agent telemetry state with individual metric histories.
#[derive(Debug, Clone)]
pub struct AgentTelemetry {
    latest: Option<TelemetryData>,
    enabled: bool,
    interval: i32,
    cpu_usage: RingBuffer<f64>,
    cpu_temp: RingBuffer<f64>,
    gpu_usage: RingBuffer<f64>,
    gpu_temp: RingBuffer<f64>,
    mem_usage: RingBuffer<f64>,
}

impl AgentTelemetry {
    /// Create a new per-agent state with the given history capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            latest: None,
            enabled: false,
            interval: 0,
            cpu_usage: RingBuffer::new(capacity),
            cpu_temp: RingBuffer::new(capacity),
            gpu_usage: RingBuffer::new(capacity),
            gpu_temp: RingBuffer::new(capacity),
            mem_usage: RingBuffer::new(capacity),
        }
    }

    /// Ingest a telemetry data snapshot: store as latest and push metrics.
    pub fn process_data(&mut self, data: &TelemetryData) {
        if let Some(cpu) = &data.cpu {
            self.cpu_usage.push(cpu.usage_percent);
            self.cpu_temp.push(cpu.temp_celsius);
        }
        if let Some(gpu) = &data.gpu {
            self.gpu_usage.push(gpu.usage_percent);
            self.gpu_temp.push(gpu.temp_celsius);
        }
        if let Some(mem) = &data.memory {
            self.mem_usage.push(mem.usage_percent);
        }
        self.latest = Some(data.clone());
    }

    /// Update telemetry enabled/interval from a status event.
    pub fn process_status(&mut self, event: &TelemetryStatusEvent) {
        self.enabled = event.enabled;
        self.interval = event.interval;
    }

    /// The most recent full telemetry snapshot, if any.
    pub fn latest(&self) -> Option<&TelemetryData> {
        self.latest.as_ref()
    }

    /// Whether the agent has telemetry enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Telemetry reporting interval in seconds.
    pub fn interval(&self) -> i32 {
        self.interval
    }

    /// CPU usage percentage history.
    pub fn cpu_usage_history(&self) -> &RingBuffer<f64> {
        &self.cpu_usage
    }

    /// CPU temperature history.
    pub fn cpu_temp_history(&self) -> &RingBuffer<f64> {
        &self.cpu_temp
    }

    /// GPU usage percentage history.
    pub fn gpu_usage_history(&self) -> &RingBuffer<f64> {
        &self.gpu_usage
    }

    /// GPU temperature history.
    pub fn gpu_temp_history(&self) -> &RingBuffer<f64> {
        &self.gpu_temp
    }

    /// Memory usage percentage history.
    pub fn mem_usage_history(&self) -> &RingBuffer<f64> {
        &self.mem_usage
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
    pub fn process_data(&mut self, agent_id: &str, data: &TelemetryData) {
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
    use capydeploy_protocol::telemetry::{CpuMetrics, GpuMetrics, MemoryMetrics};

    use super::*;

    fn sample_data_full() -> TelemetryData {
        TelemetryData {
            timestamp: 1700000000,
            cpu: Some(CpuMetrics {
                usage_percent: 45.0,
                temp_celsius: 65.0,
                freq_m_hz: 3200.0,
            }),
            gpu: Some(GpuMetrics {
                usage_percent: 80.0,
                temp_celsius: 75.0,
                freq_m_hz: 1800.0,
                mem_freq_m_hz: 0.0,
                vram_used_bytes: 0,
                vram_total_bytes: 0,
            }),
            memory: Some(MemoryMetrics {
                total_bytes: 16_000_000_000,
                available_bytes: 8_000_000_000,
                usage_percent: 50.0,
                swap_total_bytes: 0,
                swap_free_bytes: 0,
            }),
            battery: None,
            power: None,
            fan: None,
            steam: None,
        }
    }

    fn sample_data_cpu_only() -> TelemetryData {
        TelemetryData {
            timestamp: 1700000001,
            cpu: Some(CpuMetrics {
                usage_percent: 30.0,
                temp_celsius: 55.0,
                freq_m_hz: 2800.0,
            }),
            gpu: None,
            memory: None,
            battery: None,
            power: None,
            fan: None,
            steam: None,
        }
    }

    // --- AgentTelemetry tests ---

    #[test]
    fn process_data_full_metrics() {
        let mut agent = AgentTelemetry::new(10);
        let data = sample_data_full();

        agent.process_data(&data);

        assert_eq!(agent.latest().unwrap().timestamp, 1700000000);
        assert_eq!(agent.cpu_usage_history().len(), 1);
        assert_eq!(agent.cpu_temp_history().len(), 1);
        assert_eq!(agent.gpu_usage_history().len(), 1);
        assert_eq!(agent.gpu_temp_history().len(), 1);
        assert_eq!(agent.mem_usage_history().len(), 1);

        assert_eq!(agent.cpu_usage_history().last(), Some(&45.0));
        assert_eq!(agent.gpu_temp_history().last(), Some(&75.0));
        assert_eq!(agent.mem_usage_history().last(), Some(&50.0));
    }

    #[test]
    fn process_data_partial_metrics() {
        let mut agent = AgentTelemetry::new(10);
        let data = sample_data_cpu_only();

        agent.process_data(&data);

        assert_eq!(agent.cpu_usage_history().len(), 1);
        assert_eq!(agent.cpu_temp_history().len(), 1);
        assert!(agent.gpu_usage_history().is_empty());
        assert!(agent.gpu_temp_history().is_empty());
        assert!(agent.mem_usage_history().is_empty());
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
            let data = TelemetryData {
                timestamp: 1700000000 + i,
                cpu: Some(CpuMetrics {
                    usage_percent: i as f64 * 10.0,
                    temp_celsius: 50.0 + i as f64,
                    freq_m_hz: 3000.0,
                }),
                gpu: None,
                memory: None,
                battery: None,
                power: None,
                fan: None,
                steam: None,
            };
            agent.process_data(&data);
        }

        assert_eq!(agent.cpu_usage_history().len(), 5);
        let values: Vec<&f64> = agent.cpu_usage_history().iter().collect();
        assert_eq!(values, vec![&0.0, &10.0, &20.0, &30.0, &40.0]);
    }

    // --- TelemetryHub tests ---

    #[test]
    fn multiple_agents_tracked_independently() {
        let mut hub = TelemetryHub::with_capacity(10);

        hub.process_data("agent-1", &sample_data_full());
        hub.process_data("agent-2", &sample_data_cpu_only());

        let a1 = hub.get_agent("agent-1").unwrap();
        assert_eq!(a1.gpu_usage_history().len(), 1);

        let a2 = hub.get_agent("agent-2").unwrap();
        assert!(a2.gpu_usage_history().is_empty());
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
