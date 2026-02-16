//! Hardware telemetry collector for the CapyDeploy agent.
//!
//! Periodically reads system metrics (CPU, GPU, memory, battery, power, fan)
//! from platform-specific sources and delivers them via a callback.

mod collector;

#[cfg(target_os = "linux")]
#[path = "sysfs_linux.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "sysfs_windows.rs"]
mod platform;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[path = "sysfs_other.rs"]
mod platform;

pub use collector::Collector;
