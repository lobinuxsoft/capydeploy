//! Console log batch collector with reconnection and level filtering.
//!
//! Manages the CDP connection lifecycle, accumulates entries in a ring buffer,
//! and flushes batches to a callback at regular intervals.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use capydeploy_protocol::console_log::{ConsoleLogBatch, ConsoleLogEntry};
use capydeploy_protocol::constants::LOG_LEVEL_DEFAULT;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::cdp;

/// Maximum number of entries kept in the ring buffer.
const MAX_BUFFER_SIZE: usize = 200;

/// Maximum entries sent per flush.
const MAX_BATCH_SIZE: usize = 50;

/// How often the buffer is flushed.
const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

/// Backoff delays between reconnection attempts.
const BACKOFFS: &[Duration] = &[
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

/// Callback invoked with each console log batch.
pub type SendFn = Box<dyn Fn(ConsoleLogBatch) + Send + Sync + 'static>;

/// CDP-based console log collector.
///
/// Connects to Steam's CEF debugger, streams console events, and delivers
/// them in batches through the configured callback.
pub struct Collector {
    inner: Arc<Mutex<CollectorState>>,
    level_mask: Arc<AtomicU32>,
}

struct CollectorState {
    send_fn: SendFn,
    cancel: Option<CancellationToken>,
    buffer: VecDeque<ConsoleLogEntry>,
    dropped: i32,
}

impl Collector {
    /// Creates a new collector with the given batch callback.
    pub fn new(send_fn: SendFn) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CollectorState {
                send_fn,
                cancel: None,
                buffer: VecDeque::new(),
                dropped: 0,
            })),
            level_mask: Arc::new(AtomicU32::new(LOG_LEVEL_DEFAULT)),
        }
    }

    /// Starts the console log collector.
    ///
    /// Idempotent: does nothing if already running.
    pub async fn start(&self) {
        let mut state = self.inner.lock().await;
        if state.cancel.is_some() {
            return; // Already running.
        }

        let cancel = CancellationToken::new();
        state.cancel = Some(cancel.clone());

        let inner = Arc::clone(&self.inner);
        let mask = Arc::clone(&self.level_mask);

        tokio::spawn(async move {
            collection_loop(inner, mask, cancel).await;
        });

        tracing::info!("console log collector started");
    }

    /// Stops the console log collector.
    pub async fn stop(&self) {
        let mut state = self.inner.lock().await;
        if let Some(cancel) = state.cancel.take() {
            cancel.cancel();
            state.buffer.clear();
            state.dropped = 0;
            tracing::info!("console log collector stopped");
        }
    }

    /// Returns `true` if the collector is running.
    pub async fn is_running(&self) -> bool {
        self.inner.lock().await.cancel.is_some()
    }

    /// Returns the current level filter bitmask.
    pub fn get_level_mask(&self) -> u32 {
        self.level_mask.load(Ordering::Relaxed)
    }

    /// Sets the level filter bitmask. Takes effect immediately without reconnection.
    pub fn set_level_mask(&self, mask: u32) {
        self.level_mask.store(mask, Ordering::Relaxed);
        tracing::debug!(mask, "console log level mask updated");
    }
}

/// Main collection loop with reconnection logic.
async fn collection_loop(
    inner: Arc<Mutex<CollectorState>>,
    level_mask: Arc<AtomicU32>,
    cancel: CancellationToken,
) {
    for attempt in 0u32.. {
        if cancel.is_cancelled() {
            break;
        }

        // Backoff delay (skip first attempt).
        if attempt > 0 {
            let idx = (attempt as usize - 1).min(BACKOFFS.len() - 1);
            let delay = BACKOFFS[idx];
            tracing::debug!(?delay, attempt, "console log reconnect backoff");

            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(delay) => {}
            }
        }

        // Discover the CDP WebSocket URL.
        let ws_url = match cdp::discover_ws_url().await {
            Ok(url) => url,
            Err(e) => {
                tracing::warn!(error = %e, "failed to discover CEF tab");
                if attempt >= BACKOFFS.len() as u32 {
                    tracing::error!("giving up after {} reconnection attempts", attempt + 1);
                    break;
                }
                continue;
            }
        };

        // Spawn the flush ticker alongside the CDP stream.
        let flush_cancel = CancellationToken::new();
        let flush_inner = Arc::clone(&inner);
        let flush_token = flush_cancel.clone();

        let flush_task = tokio::spawn(async move {
            flush_loop(flush_inner, flush_token).await;
        });

        // Stream CDP events. Entries are pushed into the buffer via on_entry.
        let inner_ref = Arc::clone(&inner);
        let mask_ref = Arc::clone(&level_mask);

        let result = cdp::stream_events(
            &ws_url,
            move || mask_ref.load(Ordering::Relaxed),
            cancel.clone(),
            move |entry| {
                // Non-async push: try_lock to avoid blocking the CDP reader.
                if let Ok(mut state) = inner_ref.try_lock() {
                    add_entry(&mut state, entry);
                }
            },
        )
        .await;

        // Stop flush ticker and do a final flush.
        flush_cancel.cancel();
        let _ = flush_task.await;
        flush_once(&inner).await;

        match result {
            cdp::StreamEnd::Cancelled => break,
            cdp::StreamEnd::Error(e) => {
                tracing::warn!(error = %e, "CDP stream ended");
                if attempt >= BACKOFFS.len() as u32 {
                    tracing::error!("giving up after {} reconnection attempts", attempt + 1);
                    break;
                }
            }
        }
    }

    // Mark as stopped.
    let mut state = inner.lock().await;
    state.cancel = None;
}

/// Periodic flush loop.
async fn flush_loop(inner: Arc<Mutex<CollectorState>>, cancel: CancellationToken) {
    let mut ticker = tokio::time::interval(FLUSH_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ticker.tick().await; // Skip immediate tick.

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = ticker.tick() => {
                flush_once(&inner).await;
            }
        }
    }
}

/// Flushes up to `MAX_BATCH_SIZE` entries from the buffer.
async fn flush_once(inner: &Arc<Mutex<CollectorState>>) {
    let mut state = inner.lock().await;
    if state.buffer.is_empty() {
        return;
    }

    let count = state.buffer.len().min(MAX_BATCH_SIZE);
    let entries: Vec<ConsoleLogEntry> = state.buffer.drain(..count).collect();
    let dropped = state.dropped;
    state.dropped = 0;

    let batch = ConsoleLogBatch { entries, dropped };
    (state.send_fn)(batch);
}

/// Adds an entry to the ring buffer, dropping the oldest if full.
fn add_entry(state: &mut CollectorState, entry: ConsoleLogEntry) {
    if state.buffer.len() >= MAX_BUFFER_SIZE {
        state.buffer.pop_front();
        state.dropped += 1;
    }
    state.buffer.push_back(entry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI32;

    #[test]
    fn add_entry_within_limit() {
        let mut state = CollectorState {
            send_fn: Box::new(|_| {}),
            cancel: None,
            buffer: VecDeque::new(),
            dropped: 0,
        };

        for i in 0..10 {
            add_entry(
                &mut state,
                ConsoleLogEntry {
                    timestamp: i,
                    level: "log".into(),
                    source: "console".into(),
                    text: format!("msg {i}"),
                    url: String::new(),
                    line: 0,
                    segments: vec![],
                },
            );
        }

        assert_eq!(state.buffer.len(), 10);
        assert_eq!(state.dropped, 0);
    }

    #[test]
    fn add_entry_drops_oldest_when_full() {
        let mut state = CollectorState {
            send_fn: Box::new(|_| {}),
            cancel: None,
            buffer: VecDeque::new(),
            dropped: 0,
        };

        // Fill to max.
        for i in 0..MAX_BUFFER_SIZE {
            add_entry(
                &mut state,
                ConsoleLogEntry {
                    timestamp: i as i64,
                    level: "log".into(),
                    source: "console".into(),
                    text: format!("msg {i}"),
                    url: String::new(),
                    line: 0,
                    segments: vec![],
                },
            );
        }

        assert_eq!(state.buffer.len(), MAX_BUFFER_SIZE);
        assert_eq!(state.dropped, 0);

        // Add one more — oldest should be dropped.
        add_entry(
            &mut state,
            ConsoleLogEntry {
                timestamp: 999,
                level: "log".into(),
                source: "console".into(),
                text: "overflow".into(),
                url: String::new(),
                line: 0,
                segments: vec![],
            },
        );

        assert_eq!(state.buffer.len(), MAX_BUFFER_SIZE);
        assert_eq!(state.dropped, 1);
        assert_eq!(state.buffer[0].timestamp, 1); // msg 0 was dropped.
        assert_eq!(state.buffer.back().unwrap().text, "overflow");
    }

    #[tokio::test]
    async fn flush_once_sends_batch() {
        let batch_count = Arc::new(AtomicI32::new(0));
        let batch_count2 = Arc::clone(&batch_count);

        let inner = Arc::new(Mutex::new(CollectorState {
            send_fn: Box::new(move |batch| {
                assert!(!batch.entries.is_empty());
                batch_count2.fetch_add(1, Ordering::SeqCst);
            }),
            cancel: None,
            buffer: VecDeque::from([ConsoleLogEntry {
                timestamp: 1,
                level: "log".into(),
                source: "console".into(),
                text: "test".into(),
                url: String::new(),
                line: 0,
                segments: vec![],
            }]),
            dropped: 3,
        }));

        flush_once(&inner).await;

        assert_eq!(batch_count.load(Ordering::SeqCst), 1);
        let state = inner.lock().await;
        assert!(state.buffer.is_empty());
        assert_eq!(state.dropped, 0);
    }

    #[tokio::test]
    async fn flush_once_noop_on_empty() {
        let called = Arc::new(AtomicI32::new(0));
        let called2 = Arc::clone(&called);

        let inner = Arc::new(Mutex::new(CollectorState {
            send_fn: Box::new(move |_| {
                called2.fetch_add(1, Ordering::SeqCst);
            }),
            cancel: None,
            buffer: VecDeque::new(),
            dropped: 0,
        }));

        flush_once(&inner).await;
        assert_eq!(called.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn flush_respects_max_batch_size() {
        let entries_sent = Arc::new(AtomicI32::new(0));
        let entries_sent2 = Arc::clone(&entries_sent);

        let mut buffer = VecDeque::new();
        for i in 0..80 {
            buffer.push_back(ConsoleLogEntry {
                timestamp: i,
                level: "log".into(),
                source: "console".into(),
                text: format!("msg {i}"),
                url: String::new(),
                line: 0,
                segments: vec![],
            });
        }

        let inner = Arc::new(Mutex::new(CollectorState {
            send_fn: Box::new(move |batch| {
                entries_sent2.fetch_add(batch.entries.len() as i32, Ordering::SeqCst);
            }),
            cancel: None,
            buffer,
            dropped: 0,
        }));

        // First flush: should send MAX_BATCH_SIZE (50).
        flush_once(&inner).await;
        assert_eq!(entries_sent.load(Ordering::SeqCst), MAX_BATCH_SIZE as i32);

        // 30 remaining.
        let state = inner.lock().await;
        assert_eq!(state.buffer.len(), 30);
    }

    #[tokio::test]
    async fn collector_start_stop() {
        let collector = Collector::new(Box::new(|_| {}));
        assert!(!collector.is_running().await);

        collector.start().await;
        assert!(collector.is_running().await);

        // Start again should be idempotent.
        collector.start().await;
        assert!(collector.is_running().await);

        collector.stop().await;
        // Give the spawned task time to see the cancellation.
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!collector.is_running().await);
    }

    #[tokio::test]
    async fn collector_stop_when_not_running() {
        let collector = Collector::new(Box::new(|_| {}));
        collector.stop().await; // Should not panic.
    }

    #[test]
    fn collector_level_mask() {
        let collector = Collector::new(Box::new(|_| {}));
        assert_eq!(collector.get_level_mask(), LOG_LEVEL_DEFAULT);

        collector.set_level_mask(0xFF);
        assert_eq!(collector.get_level_mask(), 0xFF);

        collector.set_level_mask(0);
        assert_eq!(collector.get_level_mask(), 0);
    }

    #[test]
    fn constants() {
        assert_eq!(MAX_BUFFER_SIZE, 200);
        assert_eq!(MAX_BATCH_SIZE, 50);
        assert_eq!(FLUSH_INTERVAL, Duration::from_millis(500));
    }
}
