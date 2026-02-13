use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use capydeploy_protocol::types::UploadProgress;

use crate::UploadSession;

/// Default progress notification interval.
const DEFAULT_INTERVAL: Duration = Duration::from_millis(500);

/// Callback invoked with upload progress.
pub type ProgressCallback = Box<dyn Fn(UploadProgress) + Send + Sync>;

/// Tracks multiple upload sessions and notifies callbacks periodically.
pub struct ProgressTracker {
    inner: Arc<RwLock<TrackerInner>>,
    stop: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

struct TrackerInner {
    callbacks: Vec<ProgressCallback>,
    sessions: HashMap<String, Arc<UploadSession>>,
    interval: Duration,
}

impl ProgressTracker {
    /// Creates a new tracker with the given notification interval.
    ///
    /// If `interval` is `None`, defaults to 500 ms.
    pub fn new(interval: Option<Duration>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(TrackerInner {
                callbacks: Vec::new(),
                sessions: HashMap::new(),
                interval: interval.unwrap_or(DEFAULT_INTERVAL),
            })),
            stop: Arc::new(Mutex::new(None)),
        }
    }

    /// Registers a progress callback.
    pub fn on_progress(&self, callback: ProgressCallback) {
        let mut inner = self.inner.write().unwrap();
        inner.callbacks.push(callback);
    }

    /// Begins tracking a session.
    pub fn track(&self, session: Arc<UploadSession>) {
        let id = session.id();
        let mut inner = self.inner.write().unwrap();
        inner.sessions.insert(id, session);
    }

    /// Stops tracking a session.
    pub fn untrack(&self, session_id: &str) {
        let mut inner = self.inner.write().unwrap();
        inner.sessions.remove(session_id);
    }

    /// Returns a tracked session by ID.
    pub fn get_session(&self, session_id: &str) -> Option<Arc<UploadSession>> {
        let inner = self.inner.read().unwrap();
        inner.sessions.get(session_id).cloned()
    }

    /// Sends a one-time progress notification for a session.
    pub fn notify_progress(&self, session_id: &str) {
        let inner = self.inner.read().unwrap();
        if let Some(session) = inner.sessions.get(session_id) {
            let progress = session.progress();
            for cb in &inner.callbacks {
                cb(progress.clone());
            }
        }
    }

    /// Starts periodic progress notifications in a background tokio task.
    ///
    /// Call [`stop`](Self::stop) to cancel.
    pub fn start(&self) {
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        {
            let mut stop = self.stop.lock().unwrap();
            // Stop any existing task.
            drop(stop.take());
            *stop = Some(tx);
        }

        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            let interval = {
                let i = inner.read().unwrap();
                i.interval
            };
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let i = inner.read().unwrap();
                        for session in i.sessions.values() {
                            if session.is_active() {
                                let progress = session.progress();
                                for cb in &i.callbacks {
                                    cb(progress.clone());
                                }
                            }
                        }
                    }
                    _ = &mut rx => {
                        break;
                    }
                }
            }
        });
    }

    /// Stops the periodic notification task.
    pub fn stop(&self) {
        let mut stop = self.stop.lock().unwrap();
        // Dropping the sender signals the task to exit.
        drop(stop.take());
    }
}

// ---------------------------------------------------------------------------
// SpeedCalculator
// ---------------------------------------------------------------------------

struct SpeedSample {
    bytes: i64,
    timestamp: Instant,
}

/// Calculates transfer speed using a sliding window of samples.
pub struct SpeedCalculator {
    inner: Mutex<SpeedInner>,
}

struct SpeedInner {
    samples: Vec<SpeedSample>,
    max_samples: usize,
    window_size: Duration,
}

impl SpeedCalculator {
    /// Creates a new calculator.
    ///
    /// - `window_size`: time window for speed calculation (default 5 s).
    /// - `max_samples`: maximum retained samples (default 100).
    pub fn new(window_size: Option<Duration>, max_samples: Option<usize>) -> Self {
        Self {
            inner: Mutex::new(SpeedInner {
                samples: Vec::new(),
                max_samples: max_samples.unwrap_or(100),
                window_size: window_size.unwrap_or(Duration::from_secs(5)),
            }),
        }
    }

    /// Records a sample of `bytes` transferred at the current instant.
    pub fn add_sample(&self, bytes: i64) {
        let mut s = self.inner.lock().unwrap();
        let now = Instant::now();
        s.samples.push(SpeedSample {
            bytes,
            timestamp: now,
        });

        // Prune samples outside the window.
        let cutoff = now - s.window_size;
        s.samples.retain(|sample| sample.timestamp >= cutoff);

        // Limit sample count.
        if s.samples.len() > s.max_samples {
            let excess = s.samples.len() - s.max_samples;
            s.samples.drain(..excess);
        }
    }

    /// Returns the average speed in bytes/second within the window.
    ///
    /// Returns 0.0 if fewer than 2 samples.
    pub fn bytes_per_second(&self) -> f64 {
        let s = self.inner.lock().unwrap();
        if s.samples.len() < 2 {
            return 0.0;
        }

        let first = &s.samples[0];
        let last = &s.samples[s.samples.len() - 1];
        let elapsed = last.timestamp.duration_since(first.timestamp);
        if elapsed.is_zero() {
            return 0.0;
        }

        let total_bytes: i64 = s.samples.iter().map(|sample| sample.bytes).sum();
        total_bytes as f64 / elapsed.as_secs_f64()
    }

    /// Estimates time remaining to transfer `remaining_bytes`.
    ///
    /// Returns `None` if speed is zero.
    pub fn eta(&self, remaining_bytes: i64) -> Option<Duration> {
        let speed = self.bytes_per_second();
        if speed <= 0.0 {
            return None;
        }
        let secs = remaining_bytes as f64 / speed;
        Some(Duration::from_secs_f64(secs))
    }

    /// Clears all recorded samples.
    pub fn reset(&self) {
        let mut s = self.inner.lock().unwrap();
        s.samples.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use capydeploy_protocol::messages::FileEntry;
    use capydeploy_protocol::types::UploadConfig;

    fn sample_session(id: &str) -> Arc<UploadSession> {
        Arc::new(UploadSession::new(
            id.into(),
            UploadConfig {
                game_name: "Test".into(),
                install_path: "/tmp".into(),
                executable: "test.exe".into(),
                launch_options: String::new(),
                tags: String::new(),
            },
            1024,
            vec![FileEntry {
                relative_path: "test.exe".into(),
                size: 1024,
            }],
        ))
    }

    #[test]
    fn tracker_track_and_untrack() {
        let tracker = ProgressTracker::new(None);
        let session = sample_session("s1");
        tracker.track(Arc::clone(&session));
        assert!(tracker.get_session("s1").is_some());

        tracker.untrack("s1");
        assert!(tracker.get_session("s1").is_none());
    }

    #[test]
    fn tracker_notify_calls_callbacks() {
        let tracker = ProgressTracker::new(None);
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let r = Arc::clone(&received);
        tracker.on_progress(Box::new(move |p| {
            r.lock().unwrap().push(p.upload_id);
        }));

        let session = sample_session("s1");
        session.start();
        tracker.track(session);
        tracker.notify_progress("s1");

        let ids = received.lock().unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "s1");
    }

    #[test]
    fn tracker_notify_no_crash_on_missing_session() {
        let tracker = ProgressTracker::new(None);
        // Should not panic.
        tracker.notify_progress("nonexistent");
    }

    #[test]
    fn speed_calculator_no_samples() {
        let calc = SpeedCalculator::new(None, None);
        assert_eq!(calc.bytes_per_second(), 0.0);
        assert!(calc.eta(1000).is_none());
    }

    #[test]
    fn speed_calculator_single_sample() {
        let calc = SpeedCalculator::new(None, None);
        calc.add_sample(100);
        // Need at least 2 samples.
        assert_eq!(calc.bytes_per_second(), 0.0);
    }

    #[test]
    fn speed_calculator_multiple_samples() {
        let calc = SpeedCalculator::new(Some(Duration::from_secs(10)), None);
        calc.add_sample(500);
        std::thread::sleep(Duration::from_millis(50));
        calc.add_sample(500);

        let speed = calc.bytes_per_second();
        // With ~50ms between samples and 1000 total bytes, speed should be
        // roughly 20000 bytes/sec, but timing is imprecise — just check > 0.
        assert!(speed > 0.0);
    }

    #[test]
    fn speed_calculator_eta() {
        let calc = SpeedCalculator::new(Some(Duration::from_secs(10)), None);
        calc.add_sample(500);
        std::thread::sleep(Duration::from_millis(50));
        calc.add_sample(500);

        let eta = calc.eta(10_000);
        assert!(eta.is_some());
        assert!(eta.unwrap().as_secs_f64() > 0.0);
    }

    #[test]
    fn speed_calculator_reset() {
        let calc = SpeedCalculator::new(None, None);
        calc.add_sample(100);
        calc.add_sample(200);
        calc.reset();
        assert_eq!(calc.bytes_per_second(), 0.0);
    }

    #[test]
    fn speed_calculator_max_samples() {
        let calc = SpeedCalculator::new(Some(Duration::from_secs(60)), Some(5));
        for i in 0..20 {
            calc.add_sample(i * 10);
        }
        let s = calc.inner.lock().unwrap();
        assert!(s.samples.len() <= 5);
    }

    #[test]
    fn speed_calculator_concurrent_access() {
        use std::thread;

        let calc = Arc::new(SpeedCalculator::new(None, None));
        let mut handles = vec![];

        for _ in 0..10 {
            let c = Arc::clone(&calc);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.add_sample(1);
                    let _ = c.bytes_per_second();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Should not panic or deadlock.
        let _ = calc.bytes_per_second();
    }
}
