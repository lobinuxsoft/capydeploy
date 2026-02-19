//! Log file tailer using `notify` for file watching.
//!
//! Monitors game log files and streams new lines to a callback.
//! Lines from stdout are classified as "info", stderr lines as "error".

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use capydeploy_protocol::console_log::ConsoleLogEntry;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Callback invoked with new log lines.
pub type OnLinesFn = Box<dyn Fn(u32, Vec<ConsoleLogEntry>) + Send + Sync + 'static>;

/// Watches game log files and streams new lines.
pub struct LogTailer {
    inner: Arc<Mutex<TailerState>>,
}

struct TailerState {
    /// Active tails: appID → tail context.
    tails: HashMap<u32, TailContext>,
    /// Callback for new lines.
    on_lines: OnLinesFn,
    /// Root cancellation token.
    cancel: Option<CancellationToken>,
}

struct TailContext {
    /// Cancellation token for this specific tail/watcher.
    cancel: CancellationToken,
}

/// Interval between log directory polls in `start_watch`.
const WATCH_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

impl LogTailer {
    /// Creates a new log tailer with the given line callback.
    pub fn new(on_lines: OnLinesFn) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TailerState {
                tails: HashMap::new(),
                on_lines,
                cancel: None,
            })),
        }
    }

    /// Starts tailing a log file for the given appID.
    ///
    /// If already tailing this appID, the previous tail is stopped first.
    pub async fn start_tail(&self, app_id: u32, log_path: PathBuf) {
        let mut state = self.inner.lock().await;

        // Stop existing tail for this app if any.
        if let Some(ctx) = state.tails.remove(&app_id) {
            ctx.cancel.cancel();
            tracing::debug!(app_id, "stopped previous tail");
        }

        let cancel = CancellationToken::new();
        state.tails.insert(
            app_id,
            TailContext {
                cancel: cancel.clone(),
            },
        );

        let inner = Arc::clone(&self.inner);

        tokio::spawn(async move {
            tail_file(app_id, &log_path, cancel, inner).await;
        });

        tracing::info!(app_id, "started tailing log file");
    }

    /// Starts watching for new log files for the given appID.
    ///
    /// Polls `find_latest_log()` every 2 seconds. When a new file appears,
    /// starts tailing it (stopping any previous tail for this appID).
    /// Cancelled by `stop_tail(app_id)` or `stop_all()`.
    pub async fn start_watch(&self, app_id: u32, log_dir: PathBuf) {
        let mut state = self.inner.lock().await;

        // Stop existing watcher/tail for this app if any.
        if let Some(ctx) = state.tails.remove(&app_id) {
            ctx.cancel.cancel();
            tracing::debug!(app_id, "stopped previous watcher");
        }

        let cancel = CancellationToken::new();
        state.tails.insert(
            app_id,
            TailContext {
                cancel: cancel.clone(),
            },
        );

        let inner = Arc::clone(&self.inner);

        tokio::spawn(async move {
            watch_and_tail(app_id, log_dir, cancel, inner).await;
        });

        tracing::info!(app_id, "started log watcher");
    }

    /// Stops tailing the log file for the given appID.
    pub async fn stop_tail(&self, app_id: u32) {
        let mut state = self.inner.lock().await;
        if let Some(ctx) = state.tails.remove(&app_id) {
            ctx.cancel.cancel();
            tracing::info!(app_id, "stopped tailing log file");
        }
    }

    /// Stops all active tails.
    pub async fn stop_all(&self) {
        let mut state = self.inner.lock().await;
        for (app_id, ctx) in state.tails.drain() {
            ctx.cancel.cancel();
            tracing::debug!(app_id, "stopped tail");
        }
        if let Some(cancel) = state.cancel.take() {
            cancel.cancel();
        }
    }

    /// Returns the number of active tails.
    pub async fn active_count(&self) -> usize {
        self.inner.lock().await.tails.len()
    }

    /// Returns whether a specific appID is being tailed.
    pub async fn is_tailing(&self, app_id: u32) -> bool {
        self.inner.lock().await.tails.contains_key(&app_id)
    }
}

/// Watches a log directory for new files matching an appID, then tails the latest one.
///
/// When a new file appears (different from the one currently being tailed),
/// the previous tail is stopped and a new one starts. The outer watcher loop
/// runs until the parent `cancel` token is cancelled.
async fn watch_and_tail(
    app_id: u32,
    log_dir: PathBuf,
    cancel: CancellationToken,
    inner: Arc<Mutex<TailerState>>,
) {
    let mut current_file: Option<PathBuf> = None;
    let mut tail_cancel: Option<CancellationToken> = None;
    let mut poll_interval = tokio::time::interval(WATCH_POLL_INTERVAL);
    poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = poll_interval.tick() => {
                let latest = find_latest_log(&log_dir, app_id);

                if latest.as_ref() != current_file.as_ref() {
                    // Stop previous tail if running.
                    if let Some(tc) = tail_cancel.take() {
                        tc.cancel();
                    }

                    if let Some(ref path) = latest {
                        tracing::info!(
                            app_id,
                            file = %path.display(),
                            "new log file detected, starting tail"
                        );

                        let child_cancel = CancellationToken::new();
                        tail_cancel = Some(child_cancel.clone());
                        let inner2 = Arc::clone(&inner);
                        let path2 = path.clone();

                        tokio::spawn(async move {
                            tail_file(app_id, &path2, child_cancel, inner2).await;
                        });
                    }

                    current_file = latest;
                }
            }
        }
    }

    // Clean up: stop active tail.
    if let Some(tc) = tail_cancel.take() {
        tc.cancel();
    }

    // Remove from active tails.
    let mut state = inner.lock().await;
    state.tails.remove(&app_id);
}

/// Watches a single log file and emits new lines.
async fn tail_file(
    app_id: u32,
    path: &Path,
    cancel: CancellationToken,
    inner: Arc<Mutex<TailerState>>,
) {
    // Wait for the file to appear (game may not have started writing yet).
    let file = loop {
        if cancel.is_cancelled() {
            return;
        }
        match std::fs::File::open(path) {
            Ok(f) => break f,
            Err(_) => {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
                }
            }
        }
    };

    let mut reader = BufReader::new(file);

    // Seek to end — we only want new content.
    if reader.seek(SeekFrom::End(0)).is_err() {
        tracing::warn!(app_id, "failed to seek to end of log file");
    }

    let mut poll_interval = tokio::time::interval(std::time::Duration::from_millis(500));
    poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = poll_interval.tick() => {
                let lines = read_new_lines(&mut reader, app_id);
                if !lines.is_empty() {
                    let state = inner.lock().await;
                    (state.on_lines)(app_id, lines);
                }
            }
        }
    }

    // Clean up: remove from active tails.
    let mut state = inner.lock().await;
    state.tails.remove(&app_id);
}

/// Reads all new lines from the file since the last read position.
fn read_new_lines(reader: &mut BufReader<std::fs::File>, app_id: u32) -> Vec<ConsoleLogEntry> {
    let mut entries = Vec::new();
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF — no more data.
            Ok(_) => {
                let text = line.trim_end().to_string();
                if text.is_empty() {
                    continue;
                }

                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);

                // Classify level based on common patterns.
                let level = classify_level(&text);

                entries.push(ConsoleLogEntry {
                    timestamp,
                    level: level.into(),
                    source: format!("game:{app_id}"),
                    text,
                    url: String::new(),
                    line: 0,
                    segments: vec![],
                });
            }
            Err(e) => {
                tracing::warn!(app_id, error = %e, "error reading log file");
                break;
            }
        }
    }

    entries
}

/// Classifies a log line's level based on common error patterns.
fn classify_level(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if lower.contains("error") || lower.contains("fatal") || lower.contains("panic") {
        "error"
    } else if lower.contains("warn") {
        "warn"
    } else if lower.contains("debug") || lower.contains("trace") {
        "debug"
    } else {
        "info"
    }
}

/// Finds the most recent log file for a given appID in the log directory.
pub fn find_latest_log(log_dir: &Path, app_id: u32) -> Option<PathBuf> {
    let pattern = crate::wrapper::log_file_pattern(app_id);
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(log_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&pattern) && n.ends_with(".log"))
        })
        .collect();

    // Sort by name descending (timestamp in name ensures chronological order).
    candidates.sort_unstable_by(|a, b| b.cmp(a));
    candidates.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_level_error_patterns() {
        assert_eq!(classify_level("ERROR: something broke"), "error");
        assert_eq!(classify_level("Fatal error occurred"), "error");
        assert_eq!(classify_level("PANIC: oh no"), "error");
    }

    #[test]
    fn classify_level_warn() {
        assert_eq!(classify_level("WARNING: low memory"), "warn");
        assert_eq!(classify_level("Warn: deprecated API"), "warn");
    }

    #[test]
    fn classify_level_debug() {
        assert_eq!(classify_level("DEBUG: variable = 42"), "debug");
        assert_eq!(classify_level("TRACE: entering function"), "debug");
    }

    #[test]
    fn classify_level_info_default() {
        assert_eq!(classify_level("Game started successfully"), "info");
        assert_eq!(classify_level("Loading assets..."), "info");
    }

    #[test]
    fn find_latest_log_selects_newest() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        // Create log files with different timestamps.
        std::fs::write(dir.join("game_123_20260101_120000.log"), "old").unwrap();
        std::fs::write(dir.join("game_123_20260102_120000.log"), "new").unwrap();
        std::fs::write(dir.join("game_456_20260103_120000.log"), "other").unwrap();

        let latest = find_latest_log(dir, 123).unwrap();
        assert!(
            latest
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("20260102")
        );
    }

    #[test]
    fn find_latest_log_no_matches() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(find_latest_log(tmp.path(), 999).is_none());
    }

    #[test]
    fn find_latest_log_ignores_non_log_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("game_123_20260101_120000.txt"), "nope").unwrap();
        assert!(find_latest_log(tmp.path(), 123).is_none());
    }

    #[tokio::test]
    async fn tailer_start_stop() {
        let tailer = LogTailer::new(Box::new(|_, _| {}));
        assert_eq!(tailer.active_count().await, 0);

        let tmp = tempfile::NamedTempFile::new().unwrap();
        tailer.start_tail(123, tmp.path().to_path_buf()).await;
        assert!(tailer.is_tailing(123).await);
        assert_eq!(tailer.active_count().await, 1);

        tailer.stop_tail(123).await;
        // Give the spawned task time to clean up.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(!tailer.is_tailing(123).await);
    }

    #[tokio::test]
    async fn tailer_stop_all() {
        let tailer = LogTailer::new(Box::new(|_, _| {}));

        let tmp1 = tempfile::NamedTempFile::new().unwrap();
        let tmp2 = tempfile::NamedTempFile::new().unwrap();

        tailer.start_tail(1, tmp1.path().to_path_buf()).await;
        tailer.start_tail(2, tmp2.path().to_path_buf()).await;
        assert_eq!(tailer.active_count().await, 2);

        tailer.stop_all().await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn tailer_reads_new_lines() {
        use std::io::Write;
        use std::sync::atomic::{AtomicI32, Ordering};

        let line_count = Arc::new(AtomicI32::new(0));
        let line_count2 = Arc::clone(&line_count);

        let tailer = LogTailer::new(Box::new(move |_app_id, lines| {
            line_count2.fetch_add(lines.len() as i32, Ordering::SeqCst);
        }));

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        // Start tailing (will seek to end, so existing content is skipped).
        tailer.start_tail(42, path.clone()).await;

        // Write new lines after a short delay.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            writeln!(file, "line 1").unwrap();
            writeln!(file, "line 2").unwrap();
            writeln!(file, "ERROR: something failed").unwrap();
        }

        // Wait for the tailer to pick up the lines.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        tailer.stop_tail(42).await;

        let count = line_count.load(Ordering::SeqCst);
        assert!(count >= 3, "expected at least 3 lines, got {count}");
    }

    #[tokio::test]
    async fn start_watch_detects_new_log_file() {
        use std::io::Write;
        use std::sync::atomic::{AtomicI32, Ordering};

        let line_count = Arc::new(AtomicI32::new(0));
        let line_count2 = Arc::clone(&line_count);

        let tailer = LogTailer::new(Box::new(move |_app_id, lines| {
            line_count2.fetch_add(lines.len() as i32, Ordering::SeqCst);
        }));

        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().to_path_buf();

        // Start watcher before any log file exists.
        tailer.start_watch(42, log_dir.clone()).await;
        assert!(tailer.is_tailing(42).await);

        // Wait a bit, then create a log file.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let log_file = log_dir.join("game_42_20260101_120000.log");
        {
            let mut f = std::fs::File::create(&log_file).unwrap();
            writeln!(f, "hello from game").unwrap();
        }

        // Wait for watcher to detect and tail the file.
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;

        // Append more lines.
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&log_file)
                .unwrap();
            writeln!(f, "another line").unwrap();
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        tailer.stop_tail(42).await;

        let count = line_count.load(Ordering::SeqCst);
        assert!(count >= 1, "expected at least 1 line, got {count}");
    }

    #[tokio::test]
    async fn start_watch_replaces_on_stop() {
        let tailer = LogTailer::new(Box::new(|_, _| {}));
        let tmp = tempfile::tempdir().unwrap();

        tailer.start_watch(10, tmp.path().to_path_buf()).await;
        assert!(tailer.is_tailing(10).await);

        tailer.stop_tail(10).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(!tailer.is_tailing(10).await);
    }

    #[test]
    fn read_new_lines_from_buffer() {
        use std::io::Write;

        let tmp = tempfile::NamedTempFile::new().unwrap();
        {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(tmp.path())
                .unwrap();
            writeln!(file, "hello world").unwrap();
            writeln!(file, "ERROR: test error").unwrap();
            writeln!(file).unwrap(); // Empty line should be skipped.
            writeln!(file, "DEBUG: verbose").unwrap();
        }

        let file = std::fs::File::open(tmp.path()).unwrap();
        let mut reader = BufReader::new(file);

        let entries = read_new_lines(&mut reader, 1);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].level, "info");
        assert_eq!(entries[0].text, "hello world");
        assert_eq!(entries[1].level, "error");
        assert_eq!(entries[2].level, "debug");
    }
}
