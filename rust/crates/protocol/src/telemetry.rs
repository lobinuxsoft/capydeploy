use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Sent when telemetry is enabled/disabled on the Agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryStatusEvent {
    pub enabled: bool,
    pub interval: i32,
}

/// Hardware metrics from the Agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryData {
    pub timestamp: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battery: Option<BatteryMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<PowerMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fan: Option<FanMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steam: Option<SteamStatus>,
}

/// CPU usage, temperature, and frequency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuMetrics {
    #[serde(default)]
    pub usage_percent: f64,
    #[serde(default)]
    pub temp_celsius: f64,
    #[serde(default)]
    pub freq_m_hz: f64,
}

/// GPU usage, temperature, frequency, and VRAM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuMetrics {
    #[serde(default)]
    pub usage_percent: f64,
    #[serde(default)]
    pub temp_celsius: f64,
    #[serde(default)]
    pub freq_m_hz: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub mem_freq_m_hz: f64,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub vram_used_bytes: i64,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub vram_total_bytes: i64,
}

/// Memory and swap usage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetrics {
    #[serde(default)]
    pub total_bytes: i64,
    #[serde(default)]
    pub available_bytes: i64,
    #[serde(default)]
    pub usage_percent: f64,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub swap_total_bytes: i64,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub swap_free_bytes: i64,
}

/// Battery status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryMetrics {
    #[serde(default)]
    pub capacity: i32,
    #[serde(default)]
    pub status: String,
}

/// TDP and power draw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerMetrics {
    #[serde(default)]
    pub tdp_watts: f64,
    #[serde(default)]
    pub power_watts: f64,
}

/// Fan speed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FanMetrics {
    #[serde(default)]
    pub rpm: i32,
}

/// Steam process status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamStatus {
    pub running: bool,
    pub gaming_mode: bool,
}

/// Reports the game log wrapper state for all tracked games.
///
/// Go serializes `map[uint32]bool` with string keys in JSON. We use a custom
/// serde module to handle `HashMap<u32, bool>` ↔ `{"12345": true}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameLogWrapperStatusEvent {
    #[serde(with = "u32_key_map")]
    pub wrappers: HashMap<u32, bool>,
}

/// Confirms the game log wrapper state for a specific game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetGameLogWrapperResponse {
    pub app_id: u32,
    pub enabled: bool,
}

fn is_zero_f64(v: &f64) -> bool {
    *v == 0.0
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

/// Serde module for `HashMap<u32, bool>` where keys are serialized as strings.
///
/// Go's `encoding/json` always serializes integer map keys as strings.
mod u32_key_map {
    use std::collections::HashMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

    pub fn serialize<S: Serializer>(
        map: &HashMap<u32, bool>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let string_map: HashMap<String, bool> =
            map.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<u32, bool>, D::Error> {
        let string_map: HashMap<String, bool> = HashMap::deserialize(deserializer)?;
        string_map
            .into_iter()
            .map(|(k, v)| {
                k.parse::<u32>()
                    .map(|key| (key, v))
                    .map_err(|e| de::Error::custom(format!("invalid u32 key '{k}': {e}")))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_data_roundtrip() {
        let data = TelemetryData {
            timestamp: 1700000000,
            cpu: Some(CpuMetrics {
                usage_percent: 45.5,
                temp_celsius: 65.0,
                freq_m_hz: 3200.0,
            }),
            gpu: None,
            memory: Some(MemoryMetrics {
                total_bytes: 16_000_000_000,
                available_bytes: 8_000_000_000,
                usage_percent: 50.0,
                swap_total_bytes: 0,
                swap_free_bytes: 0,
            }),
            battery: None,
            power: None,
            fan: None,
            steam: Some(SteamStatus {
                running: true,
                gaming_mode: false,
            }),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(!json.contains("gpu"));
        assert!(!json.contains("battery"));
        let parsed: TelemetryData = serde_json::from_str(&json).unwrap();
        assert_eq!(data, parsed);
    }

    #[test]
    fn cpu_metrics_field_names() {
        let cpu = CpuMetrics {
            usage_percent: 10.0,
            temp_celsius: 50.0,
            freq_m_hz: 2400.0,
        };
        let json = serde_json::to_string(&cpu).unwrap();
        assert!(json.contains("\"usagePercent\""));
        assert!(json.contains("\"tempCelsius\""));
        assert!(json.contains("\"freqMHz\""));
    }

    #[test]
    fn gpu_metrics_omit_zero() {
        let gpu = GpuMetrics {
            usage_percent: 80.0,
            temp_celsius: 75.0,
            freq_m_hz: 1800.0,
            mem_freq_m_hz: 0.0,
            vram_used_bytes: 0,
            vram_total_bytes: 0,
        };
        let json = serde_json::to_string(&gpu).unwrap();
        assert!(!json.contains("memFreqMHz"));
        assert!(!json.contains("vramUsedBytes"));
    }

    #[test]
    fn game_log_wrapper_u32_keys() {
        let mut wrappers = HashMap::new();
        wrappers.insert(12345, true);
        wrappers.insert(67890, false);
        let evt = GameLogWrapperStatusEvent { wrappers };
        let json = serde_json::to_string(&evt).unwrap();
        // Keys should be strings in JSON (matching Go behavior)
        assert!(json.contains("\"12345\""));
        assert!(json.contains("\"67890\""));
        let parsed: GameLogWrapperStatusEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.wrappers.get(&12345), Some(&true));
    }

    #[test]
    fn telemetry_status_roundtrip() {
        let status = TelemetryStatusEvent {
            enabled: true,
            interval: 5,
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: TelemetryStatusEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(status, parsed);
    }
}
