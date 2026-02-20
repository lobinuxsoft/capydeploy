//! Linux sysfs/procfs metric readers.
//!
//! Paths are resolved once on first access and cached for the lifetime of
//! the process (they are stable per boot on Linux).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Path cache (resolved once via OnceLock)
// ---------------------------------------------------------------------------

struct SysfsPaths {
    cpu_temp: Option<PathBuf>,
    cpu_freq_paths: Vec<PathBuf>,
    gpu_busy: Option<PathBuf>,
    gpu_temp: Option<PathBuf>,
    gpu_freq: Option<PathBuf>,
    gpu_mem_freq: Option<PathBuf>,
    vram_used: Option<PathBuf>,
    vram_total: Option<PathBuf>,
    power_cap: Option<PathBuf>,
    power_avg: Option<PathBuf>,
    fan: Option<PathBuf>,
    battery: Option<PathBuf>,
}

static PATHS: OnceLock<SysfsPaths> = OnceLock::new();

fn cached_paths() -> &'static SysfsPaths {
    PATHS.get_or_init(resolve_paths)
}

fn resolve_paths() -> SysfsPaths {
    let mut paths = SysfsPaths {
        cpu_temp: None,
        cpu_freq_paths: Vec::new(),
        gpu_busy: None,
        gpu_temp: None,
        gpu_freq: None,
        gpu_mem_freq: None,
        vram_used: None,
        vram_total: None,
        power_cap: None,
        power_avg: None,
        fan: None,
        battery: None,
    };

    // Scan hwmon devices for CPU temp, power, and fan.
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let dir = entry.path();
            let name = read_trimmed(&dir.join("name")).unwrap_or_default();

            // CPU sensor (AMD k10temp or Intel coretemp).
            if name == "k10temp" || name == "coretemp" {
                if paths.cpu_temp.is_none() {
                    let p = dir.join("temp1_input");
                    if p.exists() {
                        paths.cpu_temp = Some(p);
                    }
                }
                if paths.power_cap.is_none() {
                    let p = dir.join("power1_cap");
                    if p.exists() {
                        paths.power_cap = Some(p);
                    }
                }
                if paths.power_avg.is_none() {
                    let p = dir.join("power1_average");
                    if p.exists() {
                        paths.power_avg = Some(p);
                    } else {
                        let p = dir.join("power1_input");
                        if p.exists() {
                            paths.power_avg = Some(p);
                        }
                    }
                }
            }

            // Fan sensor.
            if paths.fan.is_none() {
                let p = dir.join("fan1_input");
                if p.exists() {
                    paths.fan = Some(p);
                }
            }
        }
    }

    // GPU (AMD DRM card).
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        let mut cards: Vec<_> = entries
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("card") && n.len() <= 6)
            })
            .collect();
        cards.sort_by_key(|e| e.file_name());

        for card in cards {
            let dev = card.path().join("device");
            let busy = dev.join("gpu_busy_percent");
            if busy.exists() {
                paths.gpu_busy = Some(busy);

                // GPU temp: device/hwmon/hwmon*/temp1_input
                if let Ok(hwmon_entries) = std::fs::read_dir(dev.join("hwmon")) {
                    for he in hwmon_entries.flatten() {
                        let p = he.path().join("temp1_input");
                        if p.exists() {
                            paths.gpu_temp = Some(p);
                            break;
                        }
                    }
                }

                let sclk = dev.join("pp_dpm_sclk");
                if sclk.exists() {
                    paths.gpu_freq = Some(sclk);
                }
                let mclk = dev.join("pp_dpm_mclk");
                if mclk.exists() {
                    paths.gpu_mem_freq = Some(mclk);
                }
                let vram_used = dev.join("mem_info_vram_used");
                if vram_used.exists() {
                    paths.vram_used = Some(vram_used);
                }
                let vram_total = dev.join("mem_info_vram_total");
                if vram_total.exists() {
                    paths.vram_total = Some(vram_total);
                }

                break; // Use first card with GPU busy.
            }
        }
    }

    // Battery.
    if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_str().is_some_and(|n| n.starts_with("BAT")) {
                paths.battery = Some(entry.path());
                break;
            }
        }
    }

    // CPU frequency paths.
    if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
        let mut freq_paths: Vec<PathBuf> = entries
            .flatten()
            .filter(|e| {
                e.file_name().to_str().is_some_and(|n| {
                    n.starts_with("cpu") && n[3..].chars().all(|c| c.is_ascii_digit())
                })
            })
            .map(|e| e.path().join("cpufreq/scaling_cur_freq"))
            .filter(|p| p.exists())
            .collect();
        freq_paths.sort();
        paths.cpu_freq_paths = freq_paths;
    }

    paths
}

// ---------------------------------------------------------------------------
// Public metric readers
// ---------------------------------------------------------------------------

/// Reads CPU idle and total jiffies from `/proc/stat`.
pub fn read_cpu_times() -> (u64, u64) {
    let content = match std::fs::read_to_string("/proc/stat") {
        Ok(c) => c,
        Err(_) => return (0, 0),
    };

    // First line: "cpu  user nice system idle iowait irq softirq steal ..."
    let line = match content.lines().next() {
        Some(l) if l.starts_with("cpu ") => l,
        _ => return (0, 0),
    };

    let fields: Vec<u64> = line
        .split_whitespace()
        .skip(1) // skip "cpu"
        .filter_map(|f| f.parse().ok())
        .collect();

    if fields.len() < 4 {
        return (0, 0);
    }

    let idle = fields[3]; // 4th field is idle.
    let total: u64 = fields.iter().sum();
    (idle, total)
}

/// Reads CPU temperature in Celsius.
pub fn read_cpu_temp() -> f64 {
    cached_paths()
        .cpu_temp
        .as_ref()
        .and_then(|p| read_i64(p))
        .map(|v| v as f64 / 1000.0)
        .unwrap_or(-1.0)
}

/// Reads average CPU frequency in MHz.
pub fn read_cpu_freq() -> f64 {
    let paths = &cached_paths().cpu_freq_paths;
    if paths.is_empty() {
        return -1.0;
    }

    let mut sum: u64 = 0;
    let mut count: u64 = 0;
    for p in paths {
        if let Some(khz) = read_i64(p) {
            sum += khz as u64;
            count += 1;
        }
    }

    if count == 0 {
        return -1.0;
    }

    (sum / count / 1000) as f64 // kHz → MHz, truncated.
}

/// Reads GPU busy percentage.
pub fn read_gpu_usage() -> f64 {
    cached_paths()
        .gpu_busy
        .as_ref()
        .and_then(|p| read_f64(p))
        .unwrap_or(-1.0)
}

/// Reads GPU temperature in Celsius.
pub fn read_gpu_temp() -> f64 {
    cached_paths()
        .gpu_temp
        .as_ref()
        .and_then(|p| read_i64(p))
        .map(|v| v as f64 / 1000.0)
        .unwrap_or(-1.0)
}

/// Reads GPU core clock from `pp_dpm_sclk`.
pub fn read_gpu_freq() -> f64 {
    cached_paths()
        .gpu_freq
        .as_ref()
        .and_then(|p| parse_dpm_freq(p))
        .unwrap_or(-1.0)
}

/// Reads GPU memory clock from `pp_dpm_mclk`.
pub fn read_gpu_mem_freq() -> f64 {
    cached_paths()
        .gpu_mem_freq
        .as_ref()
        .and_then(|p| parse_dpm_freq(p))
        .unwrap_or(-1.0)
}

/// Reads VRAM (used_bytes, total_bytes). Returns (-1, -1) if unavailable.
pub fn read_vram() -> (i64, i64) {
    let paths = cached_paths();
    let total = paths
        .vram_total
        .as_ref()
        .and_then(|p| read_i64(p))
        .unwrap_or(-1);

    if total < 0 {
        return (-1, -1);
    }

    let used = paths
        .vram_used
        .as_ref()
        .and_then(|p| read_i64(p))
        .unwrap_or(-1);

    (used, total)
}

/// Reads memory info from `/proc/meminfo`.
/// Returns (total, available, swap_total, swap_free) in bytes.
pub fn read_mem_info() -> (i64, i64, i64, i64) {
    let content = match std::fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return (-1, -1, -1, -1),
    };

    let mut total: i64 = -1;
    let mut available: i64 = -1;
    let mut swap_total: i64 = 0;
    let mut swap_free: i64 = 0;

    for line in content.lines() {
        if let Some(val) = parse_meminfo_kb(line, "MemTotal:") {
            total = val * 1024;
        } else if let Some(val) = parse_meminfo_kb(line, "MemAvailable:") {
            available = val * 1024;
        } else if let Some(val) = parse_meminfo_kb(line, "SwapTotal:") {
            swap_total = val * 1024;
        } else if let Some(val) = parse_meminfo_kb(line, "SwapFree:") {
            swap_free = val * 1024;
        }
    }

    (total, available, swap_total, swap_free)
}

/// Reads battery capacity (0-100) and status string.
pub fn read_battery() -> (i32, String) {
    let bat_path = match &cached_paths().battery {
        Some(p) => p,
        None => return (-1, String::new()),
    };

    let capacity = read_trimmed(&bat_path.join("capacity"))
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(-1);

    if capacity < 0 {
        return (-1, String::new());
    }

    let status = read_trimmed(&bat_path.join("status")).unwrap_or_else(|| "Unknown".into());

    (capacity, status)
}

/// Reads TDP (watts) and power draw (watts).
pub fn read_power_info() -> (f64, f64) {
    let paths = cached_paths();

    let tdp = paths
        .power_cap
        .as_ref()
        .and_then(|p| read_i64(p))
        .map(|v| v as f64 / 1_000_000.0)
        .unwrap_or(-1.0);

    let power = paths
        .power_avg
        .as_ref()
        .and_then(|p| read_i64(p))
        .map(|v| v as f64 / 1_000_000.0)
        .unwrap_or(-1.0);

    (tdp, power)
}

/// Reads fan speed in RPM.
pub fn read_fan_speed() -> i32 {
    cached_paths()
        .fan
        .as_ref()
        .and_then(|p| read_i64(p))
        .map(|v| v as i32)
        .unwrap_or(-1)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Reads a file and trims whitespace.
fn read_trimmed(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Reads a file as i64.
fn read_i64(path: &Path) -> Option<i64> {
    read_trimmed(path).and_then(|s| s.parse().ok())
}

/// Reads a file as f64.
fn read_f64(path: &Path) -> Option<f64> {
    read_trimmed(path).and_then(|s| s.parse().ok())
}

/// Parses a `pp_dpm_sclk`/`pp_dpm_mclk` file to extract the active frequency.
///
/// Format:
/// ```text
/// 0: 400Mhz
/// 1: 1000Mhz
/// 2: 2500Mhz *
/// ```
/// The line with `*` is the current frequency. Falls back to the last entry.
fn parse_dpm_freq(path: &Path) -> Option<f64> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut last_freq: Option<f64> = None;

    for line in content.lines() {
        let line = line.trim();
        // Extract freq from "N: 1234Mhz" or "N: 1234Mhz *"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let freq_str = parts[1].trim_end_matches("Mhz").trim_end_matches("MHz");
            if let Ok(freq) = freq_str.parse::<f64>() {
                last_freq = Some(freq);
                if line.contains('*') {
                    return Some(freq);
                }
            }
        }
    }

    last_freq
}

/// Parses a line from `/proc/meminfo` matching a prefix, returns value in kB.
fn parse_meminfo_kb(line: &str, prefix: &str) -> Option<i64> {
    if !line.starts_with(prefix) {
        return None;
    }
    line[prefix.len()..]
        .split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_cpu_times_returns_values() {
        let (idle, total) = read_cpu_times();
        // On any Linux system, these should be > 0.
        assert!(total > 0, "total CPU jiffies should be > 0");
        assert!(idle > 0, "idle CPU jiffies should be > 0");
        assert!(idle <= total, "idle should be <= total");
    }

    #[test]
    fn read_mem_info_returns_values() {
        let (total, avail, swap_total, swap_free) = read_mem_info();
        assert!(total > 0, "total memory should be > 0");
        assert!(avail > 0, "available memory should be > 0");
        assert!(avail <= total, "available should be <= total");
        assert!(swap_total >= 0);
        assert!(swap_free >= 0);
    }

    #[test]
    fn parse_dpm_freq_active_marker() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pp_dpm_sclk");
        std::fs::write(&path, "0: 400Mhz\n1: 1000Mhz\n2: 2500Mhz *\n").unwrap();

        assert_eq!(parse_dpm_freq(&path), Some(2500.0));
    }

    #[test]
    fn parse_dpm_freq_no_marker_uses_last() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pp_dpm_sclk");
        std::fs::write(&path, "0: 400Mhz\n1: 1000Mhz\n2: 2500Mhz\n").unwrap();

        assert_eq!(parse_dpm_freq(&path), Some(2500.0));
    }

    #[test]
    fn parse_meminfo_kb_valid() {
        assert_eq!(
            parse_meminfo_kb("MemTotal:       16000000 kB", "MemTotal:"),
            Some(16000000)
        );
    }

    #[test]
    fn parse_meminfo_kb_mismatch() {
        assert_eq!(
            parse_meminfo_kb("MemAvailable:   8000000 kB", "MemTotal:"),
            None
        );
    }
}
