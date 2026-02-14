use cosmic::iced::Color;

/// CapyDeploy palette — healthy / nominal.
pub const GREEN: Color = Color::from_rgb(0.18, 0.80, 0.44);
/// CapyDeploy palette — warning zone.
pub const YELLOW: Color = Color::from_rgb(0.95, 0.77, 0.06);
/// CapyDeploy palette — critical zone.
pub const RED: Color = Color::from_rgb(0.91, 0.30, 0.24);
/// Background track for arcs and bars.
pub const TRACK_GRAY: Color = Color::from_rgb(0.25, 0.25, 0.28);
/// Subtle grid lines on sparklines.
pub const GRID_COLOR: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);

/// Threshold ratios that separate green → yellow → red zones.
#[derive(Debug, Clone, Copy)]
pub struct GaugeThresholds {
    /// Ratio where color transitions from green to yellow (0.0–1.0).
    pub warning: f32,
    /// Ratio where color transitions from yellow to red (0.0–1.0).
    pub critical: f32,
}

impl Default for GaugeThresholds {
    fn default() -> Self {
        Self {
            warning: 0.6,
            critical: 0.8,
        }
    }
}

/// Returns a solid color for the given ratio based on threshold zones.
///
/// - `[0, warning)` → green
/// - `[warning, critical)` → yellow
/// - `[critical, 1.0]` → red
pub fn color_for_ratio(ratio: f32, thresholds: &GaugeThresholds) -> Color {
    if ratio < thresholds.warning {
        GREEN
    } else if ratio < thresholds.critical {
        YELLOW
    } else {
        RED
    }
}

/// Linearly interpolate between two colors.
pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::from_rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    fn colors_eq(a: Color, b: Color) -> bool {
        approx_eq(a.r, b.r) && approx_eq(a.g, b.g) && approx_eq(a.b, b.b) && approx_eq(a.a, b.a)
    }

    #[test]
    fn color_for_ratio_green_zone() {
        let t = GaugeThresholds::default();
        assert!(colors_eq(color_for_ratio(0.0, &t), GREEN));
        assert!(colors_eq(color_for_ratio(0.3, &t), GREEN));
        assert!(colors_eq(color_for_ratio(0.59, &t), GREEN));
    }

    #[test]
    fn color_for_ratio_yellow_zone() {
        let t = GaugeThresholds::default();
        assert!(colors_eq(color_for_ratio(0.6, &t), YELLOW));
        assert!(colors_eq(color_for_ratio(0.7, &t), YELLOW));
        assert!(colors_eq(color_for_ratio(0.79, &t), YELLOW));
    }

    #[test]
    fn color_for_ratio_red_zone() {
        let t = GaugeThresholds::default();
        assert!(colors_eq(color_for_ratio(0.8, &t), RED));
        assert!(colors_eq(color_for_ratio(0.9, &t), RED));
        assert!(colors_eq(color_for_ratio(1.0, &t), RED));
    }

    #[test]
    fn color_for_ratio_custom_thresholds() {
        let t = GaugeThresholds {
            warning: 0.5,
            critical: 0.9,
        };
        assert!(colors_eq(color_for_ratio(0.49, &t), GREEN));
        assert!(colors_eq(color_for_ratio(0.5, &t), YELLOW));
        assert!(colors_eq(color_for_ratio(0.89, &t), YELLOW));
        assert!(colors_eq(color_for_ratio(0.9, &t), RED));
    }

    #[test]
    fn lerp_color_extremes() {
        let a = Color::from_rgb(0.0, 0.0, 0.0);
        let b = Color::from_rgb(1.0, 1.0, 1.0);
        assert!(colors_eq(lerp_color(a, b, 0.0), a));
        assert!(colors_eq(lerp_color(a, b, 1.0), b));
    }

    #[test]
    fn lerp_color_midpoint() {
        let a = Color::from_rgb(0.0, 0.0, 0.0);
        let b = Color::from_rgb(1.0, 1.0, 1.0);
        let mid = lerp_color(a, b, 0.5);
        assert!(approx_eq(mid.r, 0.5));
        assert!(approx_eq(mid.g, 0.5));
        assert!(approx_eq(mid.b, 0.5));
    }

    #[test]
    fn lerp_color_clamps_t() {
        let a = Color::from_rgb(0.0, 0.0, 0.0);
        let b = Color::from_rgb(1.0, 1.0, 1.0);
        assert!(colors_eq(lerp_color(a, b, -1.0), a));
        assert!(colors_eq(lerp_color(a, b, 2.0), b));
    }
}
