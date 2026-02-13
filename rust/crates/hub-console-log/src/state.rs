use std::collections::HashMap;

use capydeploy_hub_telemetry::RingBuffer;
use capydeploy_protocol::console_log::{ConsoleLogBatch, ConsoleLogEntry, ConsoleLogStatusEvent};

/// Default ring buffer capacity: 1000 entries.
///
/// Higher than telemetry (300) since log entries are discrete events,
/// not periodic samples. Agent sends batches of up to 50 entries every 500ms.
const DEFAULT_CAPACITY: usize = 1000;

/// Per-agent console log state with entry buffering and drop tracking.
#[derive(Debug, Clone)]
pub struct AgentConsoleLog {
    entries: RingBuffer<ConsoleLogEntry>,
    enabled: bool,
    level_mask: u32,
    total_dropped: i32,
}

impl AgentConsoleLog {
    /// Create a new per-agent state with the given buffer capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: RingBuffer::new(capacity),
            enabled: false,
            level_mask: 0,
            total_dropped: 0,
        }
    }

    /// Ingest a batch of console log entries and accumulate the dropped count.
    pub fn process_batch(&mut self, batch: &ConsoleLogBatch) {
        for entry in &batch.entries {
            self.entries.push(entry.clone());
        }
        self.total_dropped += batch.dropped;
    }

    /// Update enabled state and level mask from a status event.
    pub fn process_status(&mut self, event: &ConsoleLogStatusEvent) {
        self.enabled = event.enabled;
        self.level_mask = event.level_mask;
    }

    /// All buffered log entries (oldest first).
    pub fn entries(&self) -> &RingBuffer<ConsoleLogEntry> {
        &self.entries
    }

    /// Whether console log streaming is enabled for this agent.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The current log level bitmask.
    pub fn level_mask(&self) -> u32 {
        self.level_mask
    }

    /// Cumulative count of entries dropped by the agent due to overflow.
    pub fn total_dropped(&self) -> i32 {
        self.total_dropped
    }

    /// Reset buffer and dropped count.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_dropped = 0;
    }
}

/// Top-level console log state manager for all connected agents.
///
/// The Hub app feeds console log events into this struct,
/// and the iced UI reads from it. All methods are synchronous.
#[derive(Debug, Clone)]
pub struct ConsoleLogHub {
    agents: HashMap<String, AgentConsoleLog>,
    capacity: usize,
}

impl ConsoleLogHub {
    /// Create a hub with the default buffer capacity (1000 entries).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a hub with a custom per-agent buffer capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            agents: HashMap::new(),
            capacity,
        }
    }

    /// Route a console log batch to the appropriate agent state.
    pub fn process_batch(&mut self, agent_id: &str, batch: &ConsoleLogBatch) {
        self.agents
            .entry(agent_id.to_owned())
            .or_insert_with(|| AgentConsoleLog::new(self.capacity))
            .process_batch(batch);
    }

    /// Route a console log status event to the appropriate agent state.
    pub fn process_status(&mut self, agent_id: &str, event: &ConsoleLogStatusEvent) {
        self.agents
            .entry(agent_id.to_owned())
            .or_insert_with(|| AgentConsoleLog::new(self.capacity))
            .process_status(event);
    }

    /// Look up the console log state for a specific agent.
    pub fn get_agent(&self, agent_id: &str) -> Option<&AgentConsoleLog> {
        self.agents.get(agent_id)
    }

    /// Remove an agent's console log state (e.g. on disconnect).
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

impl Default for ConsoleLogHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(timestamp: i64, level: &str, text: &str) -> ConsoleLogEntry {
        ConsoleLogEntry {
            timestamp,
            level: level.into(),
            source: "console".into(),
            text: text.into(),
            url: String::new(),
            line: 0,
            segments: vec![],
        }
    }

    fn make_batch(entries: Vec<ConsoleLogEntry>, dropped: i32) -> ConsoleLogBatch {
        ConsoleLogBatch { entries, dropped }
    }

    // --- AgentConsoleLog tests ---

    #[test]
    fn process_batch_fills_buffer() {
        let mut agent = AgentConsoleLog::new(100);
        let batch = make_batch(
            vec![
                make_entry(1, "log", "hello"),
                make_entry(2, "warn", "careful"),
                make_entry(3, "error", "boom"),
            ],
            0,
        );

        agent.process_batch(&batch);

        assert_eq!(agent.entries().len(), 3);
        let texts: Vec<&str> = agent.entries().iter().map(|e| e.text.as_str()).collect();
        assert_eq!(texts, vec!["hello", "careful", "boom"]);
    }

    #[test]
    fn process_batch_accumulates_dropped() {
        let mut agent = AgentConsoleLog::new(100);

        agent.process_batch(&make_batch(vec![make_entry(1, "log", "a")], 3));
        assert_eq!(agent.total_dropped(), 3);

        agent.process_batch(&make_batch(vec![make_entry(2, "log", "b")], 7));
        assert_eq!(agent.total_dropped(), 10);

        agent.process_batch(&make_batch(vec![], 0));
        assert_eq!(agent.total_dropped(), 10);
    }

    #[test]
    fn process_status_updates_fields() {
        let mut agent = AgentConsoleLog::new(100);
        assert!(!agent.enabled());
        assert_eq!(agent.level_mask(), 0);

        agent.process_status(&ConsoleLogStatusEvent {
            enabled: true,
            level_mask: 15,
        });

        assert!(agent.enabled());
        assert_eq!(agent.level_mask(), 15);
    }

    #[test]
    fn buffer_eviction_at_capacity() {
        let mut agent = AgentConsoleLog::new(3);

        let batch = make_batch(
            vec![
                make_entry(1, "log", "first"),
                make_entry(2, "log", "second"),
                make_entry(3, "log", "third"),
                make_entry(4, "log", "fourth"),
                make_entry(5, "log", "fifth"),
            ],
            0,
        );
        agent.process_batch(&batch);

        assert_eq!(agent.entries().len(), 3);
        let texts: Vec<&str> = agent.entries().iter().map(|e| e.text.as_str()).collect();
        assert_eq!(texts, vec!["third", "fourth", "fifth"]);
    }

    #[test]
    fn clear_resets_buffer_and_dropped() {
        let mut agent = AgentConsoleLog::new(100);
        agent.process_batch(&make_batch(vec![make_entry(1, "log", "a")], 5));

        assert_eq!(agent.entries().len(), 1);
        assert_eq!(agent.total_dropped(), 5);

        agent.clear();

        assert!(agent.entries().is_empty());
        assert_eq!(agent.total_dropped(), 0);
    }

    // --- ConsoleLogHub tests ---

    #[test]
    fn multiple_agents_tracked_independently() {
        let mut hub = ConsoleLogHub::with_capacity(100);

        hub.process_batch(
            "agent-1",
            &make_batch(vec![make_entry(1, "log", "from-1")], 0),
        );
        hub.process_batch(
            "agent-2",
            &make_batch(
                vec![
                    make_entry(2, "log", "from-2a"),
                    make_entry(3, "log", "from-2b"),
                ],
                0,
            ),
        );

        assert_eq!(hub.get_agent("agent-1").unwrap().entries().len(), 1);
        assert_eq!(hub.get_agent("agent-2").unwrap().entries().len(), 2);
    }

    #[test]
    fn remove_agent_cleans_up() {
        let mut hub = ConsoleLogHub::with_capacity(100);
        hub.process_batch(
            "agent-1",
            &make_batch(vec![make_entry(1, "log", "a")], 0),
        );
        hub.process_batch(
            "agent-2",
            &make_batch(vec![make_entry(2, "log", "b")], 0),
        );

        hub.remove_agent("agent-1");

        assert!(hub.get_agent("agent-1").is_none());
        assert!(hub.get_agent("agent-2").is_some());
    }

    #[test]
    fn hub_clear_removes_all() {
        let mut hub = ConsoleLogHub::with_capacity(100);
        hub.process_batch(
            "agent-1",
            &make_batch(vec![make_entry(1, "log", "a")], 0),
        );
        hub.process_batch(
            "agent-2",
            &make_batch(vec![make_entry(2, "log", "b")], 0),
        );

        hub.clear();

        assert!(hub.agent_ids().is_empty());
    }

    #[test]
    fn agent_ids_returns_tracked() {
        let mut hub = ConsoleLogHub::with_capacity(100);
        hub.process_batch(
            "alpha",
            &make_batch(vec![make_entry(1, "log", "a")], 0),
        );
        hub.process_batch(
            "beta",
            &make_batch(vec![make_entry(2, "log", "b")], 0),
        );

        let mut ids = hub.agent_ids();
        ids.sort();
        assert_eq!(ids, vec!["alpha", "beta"]);
    }

    #[test]
    fn process_status_creates_agent_if_absent() {
        let mut hub = ConsoleLogHub::with_capacity(100);

        hub.process_status(
            "new-agent",
            &ConsoleLogStatusEvent {
                enabled: true,
                level_mask: 7,
            },
        );

        let agent = hub.get_agent("new-agent").unwrap();
        assert!(agent.enabled());
        assert_eq!(agent.level_mask(), 7);
        assert!(agent.entries().is_empty());
    }

    #[test]
    fn get_unknown_agent_returns_none() {
        let hub = ConsoleLogHub::new();
        assert!(hub.get_agent("nonexistent").is_none());
    }
}
