//! Windows metric readers.
//!
//! CPU usage, memory, and battery are implemented using Win32 APIs.
//! GPU, power, fan, and CPU temperature require vendor-specific SDKs
//! or privileged access and return -1.

use std::mem::{self, MaybeUninit};

use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::{
    Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
    SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
    Threading::GetSystemTimes,
};

/// Converts a `FILETIME` to a `u64` of 100-nanosecond intervals.
fn filetime_to_u64(ft: FILETIME) -> u64 {
    (ft.dwHighDateTime as u64) << 32 | ft.dwLowDateTime as u64
}

/// Reads CPU idle and total times via `GetSystemTimes`.
///
/// Returns `(idle_ticks, total_ticks)` in 100-nanosecond intervals.
/// The collector computes usage percentage from deltas between calls.
pub fn read_cpu_times() -> (u64, u64) {
    let mut idle = MaybeUninit::<FILETIME>::zeroed();
    let mut kernel = MaybeUninit::<FILETIME>::zeroed();
    let mut user = MaybeUninit::<FILETIME>::zeroed();

    // SAFETY: GetSystemTimes writes to the provided pointers.
    let ret = unsafe {
        GetSystemTimes(
            idle.as_mut_ptr(),
            kernel.as_mut_ptr(),
            user.as_mut_ptr(),
        )
    };

    if ret == 0 {
        return (0, 0);
    }

    // SAFETY: GetSystemTimes succeeded, all values are initialized.
    let idle = filetime_to_u64(unsafe { idle.assume_init() });
    let kernel = filetime_to_u64(unsafe { kernel.assume_init() });
    let user = filetime_to_u64(unsafe { user.assume_init() });

    // Kernel time includes idle time on Windows.
    let total = kernel + user;
    (idle, total)
}

/// Reads CPU temperature in Celsius.
pub fn read_cpu_temp() -> f64 {
    -1.0 // Requires WMI MSAcpi_ThermalZoneTemperature or Ring 0 access.
}

/// Reads average CPU frequency in MHz.
pub fn read_cpu_freq() -> f64 {
    -1.0 // TODO: CallNtPowerInformation(ProcessorInformation).
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

/// Reads memory info via `GlobalMemoryStatusEx`.
///
/// Returns `(total, available, page_file_total, page_file_available)` in bytes.
pub fn read_mem_info() -> (i64, i64, i64, i64) {
    unsafe {
        let mut status = MaybeUninit::<MEMORYSTATUSEX>::zeroed();
        // dwLength must be set before calling GlobalMemoryStatusEx.
        (*status.as_mut_ptr()).dwLength = mem::size_of::<MEMORYSTATUSEX>() as u32;

        if GlobalMemoryStatusEx(status.as_mut_ptr()) == 0 {
            return (-1, -1, -1, -1);
        }

        let s = status.assume_init();

        (
            s.ullTotalPhys as i64,
            s.ullAvailPhys as i64,
            s.ullTotalPageFile as i64,
            s.ullAvailPageFile as i64,
        )
    }
}

/// Reads battery capacity (0–100) and status string via `GetSystemPowerStatus`.
pub fn read_battery() -> (i32, String) {
    unsafe {
        let mut status = MaybeUninit::<SYSTEM_POWER_STATUS>::zeroed();

        if GetSystemPowerStatus(status.as_mut_ptr()) == 0 {
            return (-1, String::new());
        }

        let s = status.assume_init();

        // No battery present (bit 7).
        if s.BatteryFlag & 128 != 0 {
            return (-1, String::new());
        }

        // Unknown charge level.
        if s.BatteryLifePercent == 255 {
            return (-1, String::new());
        }

        let capacity = s.BatteryLifePercent as i32;

        let status_str = if s.BatteryFlag & 8 != 0 {
            "Charging"
        } else if s.ACLineStatus == 1 && capacity >= 95 {
            "Full"
        } else if s.ACLineStatus == 0 {
            "Discharging"
        } else {
            "Unknown"
        };

        (capacity, status_str.to_string())
    }
}

/// Reads TDP and power draw in watts.
pub fn read_power_info() -> (f64, f64) {
    (-1.0, -1.0)
}

/// Reads fan speed in RPM.
pub fn read_fan_speed() -> i32 {
    -1
}
