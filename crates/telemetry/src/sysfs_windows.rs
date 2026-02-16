//! Windows metric readers.
//!
//! Only CPU usage and memory are implemented using Win32 APIs.
//! GPU, power, and fan metrics require vendor-specific SDKs and return -1.

/// Reads CPU idle and total times (stub — uses `GetSystemTimes` on real Windows).
pub fn read_cpu_times() -> (u64, u64) {
    // TODO: Implement via GetSystemTimes when building on Windows.
    (0, 0)
}

pub fn read_cpu_temp() -> f64 {
    -1.0 // Requires Ring 0 MSR access.
}

pub fn read_cpu_freq() -> f64 {
    -1.0 // TODO: CallNtPowerInformation.
}

pub fn read_gpu_usage() -> f64 {
    -1.0
}

pub fn read_gpu_temp() -> f64 {
    -1.0
}

pub fn read_gpu_freq() -> f64 {
    -1.0
}

pub fn read_gpu_mem_freq() -> f64 {
    -1.0
}

pub fn read_vram() -> (i64, i64) {
    (-1, -1)
}

pub fn read_mem_info() -> (i64, i64, i64, i64) {
    // TODO: Implement via GlobalMemoryStatusEx.
    (-1, -1, -1, -1)
}

pub fn read_battery() -> (i32, String) {
    // TODO: Implement via GetSystemPowerStatus.
    (-1, String::new())
}

pub fn read_power_info() -> (f64, f64) {
    (-1.0, -1.0)
}

pub fn read_fan_speed() -> i32 {
    -1
}
