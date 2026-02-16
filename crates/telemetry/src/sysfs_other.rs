//! Stub metric readers for unsupported platforms.

pub fn read_cpu_times() -> (u64, u64) {
    (0, 0)
}
pub fn read_cpu_temp() -> f64 {
    -1.0
}
pub fn read_cpu_freq() -> f64 {
    -1.0
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
    (-1, -1, -1, -1)
}
pub fn read_battery() -> (i32, String) {
    (-1, String::new())
}
pub fn read_power_info() -> (f64, f64) {
    (-1.0, -1.0)
}
pub fn read_fan_speed() -> i32 {
    -1
}
