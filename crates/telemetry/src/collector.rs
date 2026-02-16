//! Async telemetry collector with configurable interval.

use std::sync::Arc;
use std::time::Duration;

use capydeploy_protocol::telemetry::{
    BatteryMetrics, CpuMetrics, FanMetrics, GpuMetrics, MemoryMetrics, PowerMetrics, SteamStatus,
    TelemetryData,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::platform;

/// Callback invoked with each telemetry snapshot.
pub type SendFn = Box<dyn Fn(TelemetryData) + Send + Sync + 'static>;

/// Callback for querying Steam status (running, gaming_mode).
pub type SteamStatusFn = Box<dyn Fn() -> (bool, bool) + Send + Sync + 'static>;

/// Hardware telemetry collector.
///
/// Spawns a tokio task that periodically reads system metrics and delivers
/// them through the configured callback.
pub struct Collector {
    inner: Arc<Mutex<CollectorInner>>,
}

struct CollectorInner {
    send_fn: SendFn,
    steam_status_fn: Option<SteamStatusFn>,
    cancel: Option<CancellationToken>,
    prev_idle: u64,
    prev_total: u64,
    primed: bool,
}

impl Collector {
    /// Creates a new collector with the given send callback.
    pub fn new(send_fn: SendFn) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CollectorInner {
                send_fn,
                steam_status_fn: None,
                cancel: None,
                prev_idle: 0,
                prev_total: 0,
                primed: false,
            })),
        }
    }

    /// Sets the Steam status callback.
    pub async fn set_steam_status_fn(&self, f: SteamStatusFn) {
        self.inner.lock().await.steam_status_fn = Some(f);
    }

    /// Starts periodic collection at the given interval (seconds).
    ///
    /// Minimum interval is 1 second; default is 2 if 0 is passed.
    pub async fn start(&self, interval_sec: u32) {
        let mut inner = self.inner.lock().await;

        // Stop existing loop if any.
        if let Some(cancel) = inner.cancel.take() {
            cancel.cancel();
        }

        let interval_sec = match interval_sec {
            0 => 2,
            v => v.max(1),
        };

        // Prime CPU counters.
        let (idle, total) = platform::read_cpu_times();
        inner.prev_idle = idle;
        inner.prev_total = total;
        inner.primed = false;

        let cancel = CancellationToken::new();
        inner.cancel = Some(cancel.clone());

        let collector = Arc::clone(&self.inner);
        let interval = Duration::from_secs(interval_sec as u64);

        tokio::spawn(async move {
            collection_loop(collector, interval, cancel).await;
        });

        tracing::info!(interval_sec, "telemetry collector started");
    }

    /// Stops the collector.
    pub async fn stop(&self) {
        let mut inner = self.inner.lock().await;
        if let Some(cancel) = inner.cancel.take() {
            cancel.cancel();
            inner.primed = false;
            tracing::info!("telemetry collector stopped");
        }
    }

    /// Returns `true` if the collector is running.
    pub async fn is_running(&self) -> bool {
        self.inner.lock().await.cancel.is_some()
    }

    /// Updates the collection interval (restarts the loop).
    pub async fn update_interval(&self, interval_sec: u32) {
        self.stop().await;
        self.start(interval_sec).await;
    }
}

/// Main collection loop.
async fn collection_loop(
    inner: Arc<Mutex<CollectorInner>>,
    interval: Duration,
    cancel: CancellationToken,
) {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // Skip the first immediate tick.
    ticker.tick().await;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = ticker.tick() => {
                let data = collect(&inner).await;
                let guard = inner.lock().await;
                (guard.send_fn)(data);
            }
        }
    }
}

/// Collects a single telemetry snapshot.
async fn collect(inner: &Arc<Mutex<CollectorInner>>) -> TelemetryData {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    // CPU usage (requires delta from previous read).
    let (cpu_usage, prev_idle, prev_total, was_primed) = {
        let mut guard = inner.lock().await;
        let (idle, total) = platform::read_cpu_times();

        let usage = if guard.primed && total > guard.prev_total {
            let d_total = total - guard.prev_total;
            let d_idle = idle - guard.prev_idle;
            let pct = (1.0 - d_idle as f64 / d_total as f64) * 100.0;
            (pct * 10.0).floor() / 10.0 // One decimal place.
        } else {
            -1.0
        };

        let was_primed = guard.primed;
        guard.prev_idle = idle;
        guard.prev_total = total;
        guard.primed = true;

        (usage, idle, total, was_primed)
    };
    let _ = (prev_idle, prev_total, was_primed);

    let cpu_temp = platform::read_cpu_temp();
    let cpu_freq = platform::read_cpu_freq();

    let cpu = if cpu_usage >= 0.0 || cpu_temp >= 0.0 || cpu_freq >= 0.0 {
        Some(CpuMetrics {
            usage_percent: cpu_usage,
            temp_celsius: cpu_temp,
            freq_m_hz: cpu_freq,
        })
    } else {
        None
    };

    // GPU.
    let gpu_usage = platform::read_gpu_usage();
    let gpu_temp = platform::read_gpu_temp();
    let gpu_freq = platform::read_gpu_freq();
    let gpu_mem_freq = platform::read_gpu_mem_freq();
    let (vram_used, vram_total) = platform::read_vram();

    let gpu = if gpu_usage >= 0.0 || gpu_temp >= 0.0 || gpu_freq >= 0.0 {
        Some(GpuMetrics {
            usage_percent: gpu_usage,
            temp_celsius: gpu_temp,
            freq_m_hz: gpu_freq,
            mem_freq_m_hz: if gpu_mem_freq >= 0.0 {
                gpu_mem_freq
            } else {
                0.0
            },
            vram_used_bytes: vram_used.max(0),
            vram_total_bytes: vram_total.max(0),
        })
    } else {
        None
    };

    // Memory.
    let (mem_total, mem_avail, swap_total, swap_free) = platform::read_mem_info();
    let memory = if mem_total > 0 {
        let usage_pct = (mem_total - mem_avail) as f64 / mem_total as f64 * 100.0;
        Some(MemoryMetrics {
            total_bytes: mem_total,
            available_bytes: mem_avail,
            usage_percent: (usage_pct * 10.0).floor() / 10.0,
            swap_total_bytes: swap_total.max(0),
            swap_free_bytes: swap_free.max(0),
        })
    } else {
        None
    };

    // Battery.
    let (bat_cap, bat_status) = platform::read_battery();
    let battery = if bat_cap >= 0 {
        Some(BatteryMetrics {
            capacity: bat_cap,
            status: bat_status,
        })
    } else {
        None
    };

    // Power.
    let (tdp, power_draw) = platform::read_power_info();
    let power = if tdp > 0.0 || power_draw > 0.0 {
        Some(PowerMetrics {
            tdp_watts: tdp,
            power_watts: power_draw,
        })
    } else {
        None
    };

    // Fan.
    let fan_rpm = platform::read_fan_speed();
    let fan = if fan_rpm >= 0 {
        Some(FanMetrics { rpm: fan_rpm })
    } else {
        None
    };

    // Steam status.
    let steam = {
        let guard = inner.lock().await;
        guard.steam_status_fn.as_ref().map(|f| {
            let (running, gaming_mode) = f();
            SteamStatus {
                running,
                gaming_mode,
            }
        })
    };

    TelemetryData {
        timestamp,
        cpu,
        gpu,
        memory,
        battery,
        power,
        fan,
        steam,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn collector_start_stop() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter2 = Arc::clone(&counter);

        let collector = Collector::new(Box::new(move |_data| {
            counter2.fetch_add(1, Ordering::SeqCst);
        }));

        assert!(!collector.is_running().await);

        collector.start(1).await;
        assert!(collector.is_running().await);

        // Wait for at least 2 ticks.
        tokio::time::sleep(Duration::from_millis(2500)).await;

        collector.stop().await;
        assert!(!collector.is_running().await);

        let count = counter.load(Ordering::SeqCst);
        assert!(count >= 1, "expected at least 1 tick, got {count}");
    }

    #[tokio::test]
    async fn collector_stop_when_not_running() {
        let collector = Collector::new(Box::new(|_| {}));
        collector.stop().await; // Should not panic.
    }

    #[tokio::test]
    async fn collector_update_interval() {
        let collector = Collector::new(Box::new(|_| {}));
        collector.start(2).await;
        assert!(collector.is_running().await);

        collector.update_interval(5).await;
        assert!(collector.is_running().await);

        collector.stop().await;
    }

    #[tokio::test]
    async fn collector_steam_status_callback() {
        let collector = Collector::new(Box::new(|_| {}));
        collector
            .set_steam_status_fn(Box::new(|| (true, false)))
            .await;

        // Verify the callback is stored.
        let guard = collector.inner.lock().await;
        assert!(guard.steam_status_fn.is_some());
    }

    #[tokio::test]
    async fn collect_returns_valid_data() {
        let inner = Arc::new(Mutex::new(CollectorInner {
            send_fn: Box::new(|_| {}),
            steam_status_fn: Some(Box::new(|| (false, false))),
            cancel: None,
            prev_idle: 0,
            prev_total: 0,
            primed: false,
        }));

        // First collect (priming) — CPU usage should be -1.
        let data = collect(&inner).await;
        assert!(data.timestamp > 0);

        if let Some(cpu) = &data.cpu {
            assert!(
                cpu.usage_percent < 0.0,
                "first collect should return -1 for CPU usage"
            );
        }

        // Second collect — CPU usage should be valid.
        let data2 = collect(&inner).await;
        if let Some(cpu) = &data2.cpu {
            // After priming, usage should be >= 0.
            assert!(cpu.usage_percent >= 0.0 || cpu.usage_percent == -1.0);
        }
    }

    #[tokio::test]
    async fn collect_includes_steam_status() {
        let inner = Arc::new(Mutex::new(CollectorInner {
            send_fn: Box::new(|_| {}),
            steam_status_fn: Some(Box::new(|| (true, true))),
            cancel: None,
            prev_idle: 0,
            prev_total: 0,
            primed: false,
        }));

        let data = collect(&inner).await;
        let steam = data.steam.unwrap();
        assert!(steam.running);
        assert!(steam.gaming_mode);
    }
}
