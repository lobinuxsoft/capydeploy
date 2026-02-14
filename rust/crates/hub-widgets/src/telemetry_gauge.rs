use std::f32::consts::PI;

use cosmic::iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use cosmic::iced::{alignment, mouse, Color, Point, Radians, Rectangle};

use crate::colors::{self, GaugeThresholds, TRACK_GRAY};

/// Start angle of the 270° arc (7 o'clock position).
const ARC_START: f32 = PI * 0.75;
/// Sweep of the arc in radians (270°).
const ARC_SWEEP: f32 = PI * 1.5;

/// A radial 270° arc gauge for CPU / GPU / temperature telemetry.
///
/// Renders a track arc (gray) with a colored value arc on top.
/// Center text shows the current value + unit; bottom label identifies the metric.
pub struct TelemetryGauge {
    value: f64,
    min: f64,
    max: f64,
    label: String,
    unit: String,
    thresholds: GaugeThresholds,
    cache: canvas::Cache,
}

impl TelemetryGauge {
    /// Creates a new gauge with the given label, unit, and range.
    pub fn new(label: impl Into<String>, unit: impl Into<String>, min: f64, max: f64) -> Self {
        Self {
            value: min,
            min,
            max,
            label: label.into(),
            unit: unit.into(),
            thresholds: GaugeThresholds::default(),
            cache: canvas::Cache::new(),
        }
    }

    /// Overrides the default warning / critical thresholds.
    pub fn with_thresholds(mut self, thresholds: GaugeThresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// Sets the current value, clamping to `[min, max]` and clearing the cache.
    pub fn set_value(&mut self, value: f64) {
        self.value = value.clamp(self.min, self.max);
        self.cache.clear();
    }

    /// Returns the ratio of current value within `[min, max]` as `0.0..=1.0`.
    fn ratio(&self) -> f32 {
        let range = self.max - self.min;
        if range <= 0.0 {
            return 0.0;
        }
        ((self.value - self.min) / range) as f32
    }
}

impl<Message> canvas::Program<Message, cosmic::Theme, cosmic::Renderer> for TelemetryGauge {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();
            let side = bounds.width.min(bounds.height);
            let radius = side * 0.38;
            let stroke_width = (side * 0.04).max(4.0);

            // Track arc (full 270°, gray background).
            let track = arc_path(center, radius, ARC_START, ARC_SWEEP);
            frame.stroke(
                &track,
                Stroke::default()
                    .with_width(stroke_width)
                    .with_color(TRACK_GRAY),
            );

            // Value arc (proportional to ratio).
            let ratio = self.ratio();
            if ratio > 0.0 {
                let value_sweep = ARC_SWEEP * ratio;
                let value_arc = arc_path(center, radius, ARC_START, value_sweep);
                let color = colors::color_for_ratio(ratio, &self.thresholds);
                frame.stroke(
                    &value_arc,
                    Stroke {
                        line_cap: canvas::LineCap::Round,
                        ..Stroke::default()
                            .with_width(stroke_width)
                            .with_color(color)
                    },
                );
            }

            // Center text: formatted value + unit.
            draw_center_text(frame, center, &self.value, &self.unit, side);

            // Bottom label.
            draw_label(frame, center, radius, &self.label, side);
        });

        vec![geometry]
    }
}

/// Builds a path tracing an arc from `start_angle` clockwise for `sweep` radians.
fn arc_path(center: Point, radius: f32, start_angle: f32, sweep: f32) -> Path {
    Path::new(|b| {
        let start = Point::new(
            center.x + radius * start_angle.cos(),
            center.y + radius * start_angle.sin(),
        );
        b.move_to(start);
        b.arc(canvas::path::Arc {
            center,
            radius,
            start_angle: Radians(start_angle),
            end_angle: Radians(start_angle + sweep),
        });
    })
}

/// Draws the primary value text at the center of the gauge.
fn draw_center_text(frame: &mut Frame, center: Point, value: &f64, unit: &str, side: f32) {
    let formatted = if *value == value.round() {
        format!("{:.0}{}", value, unit)
    } else {
        format!("{:.1}{}", value, unit)
    };
    let font_size = side * 0.14;

    frame.fill_text(Text {
        content: formatted,
        position: center,
        color: Color::WHITE,
        size: font_size.into(),
        horizontal_alignment: alignment::Horizontal::Center,
        vertical_alignment: alignment::Vertical::Center,
        ..Text::default()
    });
}

/// Draws the metric label below the arc.
fn draw_label(frame: &mut Frame, center: Point, radius: f32, label: &str, side: f32) {
    let font_size = side * 0.08;
    let label_y = center.y + radius * 0.55;

    frame.fill_text(Text {
        content: label.to_owned(),
        position: Point::new(center.x, label_y),
        color: Color::from_rgba(1.0, 1.0, 1.0, 0.6),
        size: font_size.into(),
        horizontal_alignment: alignment::Horizontal::Center,
        vertical_alignment: alignment::Vertical::Center,
        ..Text::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_normal() {
        let g = TelemetryGauge::new("CPU", "%", 0.0, 100.0);
        // default value is min → 0
        assert!((g.ratio() - 0.0).abs() < 1e-5);
    }

    #[test]
    fn ratio_after_set() {
        let mut g = TelemetryGauge::new("CPU", "%", 0.0, 100.0);
        g.set_value(50.0);
        assert!((g.ratio() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn ratio_clamped_above() {
        let mut g = TelemetryGauge::new("CPU", "%", 0.0, 100.0);
        g.set_value(150.0);
        assert!((g.ratio() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn ratio_clamped_below() {
        let mut g = TelemetryGauge::new("CPU", "%", 0.0, 100.0);
        g.set_value(-10.0);
        assert!((g.ratio() - 0.0).abs() < 1e-5);
    }

    #[test]
    fn ratio_zero_range() {
        let g = TelemetryGauge::new("X", "", 50.0, 50.0);
        assert!((g.ratio() - 0.0).abs() < 1e-5);
    }

    #[test]
    fn custom_thresholds() {
        let g = TelemetryGauge::new("T", "°C", 0.0, 100.0).with_thresholds(GaugeThresholds {
            warning: 0.5,
            critical: 0.9,
        });
        assert!((g.thresholds.warning - 0.5).abs() < 1e-5);
        assert!((g.thresholds.critical - 0.9).abs() < 1e-5);
    }
}
