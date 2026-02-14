pub mod console;
pub mod deploy;
pub mod devices;
pub mod telemetry;

use cosmic::iced::Color;

use capydeploy_hub_widgets::{GaugeThresholds, Sparkline, SparklineStyle, TelemetryGauge};

/// Owns the canvas widgets for the telemetry dashboard.
///
/// These must live in the `Hub` struct because each widget owns a
/// `canvas::Cache` that is mutated on `set_value`/`set_data`.
pub struct TelemetryWidgets {
    pub cpu_gauge: TelemetryGauge,
    pub gpu_gauge: TelemetryGauge,
    pub cpu_temp_gauge: TelemetryGauge,
    pub mem_gauge: TelemetryGauge,
    pub cpu_sparkline: Sparkline,
    pub gpu_sparkline: Sparkline,
    pub mem_sparkline: Sparkline,
}

impl TelemetryWidgets {
    /// Creates a new set of telemetry widgets with default configuration.
    pub fn new() -> Self {
        let temp_thresholds = GaugeThresholds {
            warning: 0.65,
            critical: 0.85,
        };

        Self {
            cpu_gauge: TelemetryGauge::new("CPU", "%", 0.0, 100.0),
            gpu_gauge: TelemetryGauge::new("GPU", "%", 0.0, 100.0),
            cpu_temp_gauge: TelemetryGauge::new("Temp", "°C", 0.0, 100.0)
                .with_thresholds(temp_thresholds),
            mem_gauge: TelemetryGauge::new("Mem", "%", 0.0, 100.0),
            cpu_sparkline: Sparkline::new(SparklineStyle::default()),
            gpu_sparkline: Sparkline::new(SparklineStyle {
                line_color: Color::from_rgb(0.18, 0.80, 0.44),
                fill_color: Color::from_rgba(0.18, 0.80, 0.44, 0.15),
                ..SparklineStyle::default()
            }),
            mem_sparkline: Sparkline::new(SparklineStyle {
                line_color: Color::from_rgb(1.0, 0.60, 0.20),
                fill_color: Color::from_rgba(1.0, 0.60, 0.20, 0.15),
                ..SparklineStyle::default()
            }),
        }
    }
}
