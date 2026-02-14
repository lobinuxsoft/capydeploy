use cosmic::iced::widget::canvas::{self, Frame, Path, Stroke};
use cosmic::iced::{mouse, Color, Point, Rectangle, Size};

use crate::colors::GRID_COLOR;

/// Visual configuration for a sparkline chart.
#[derive(Debug, Clone)]
pub struct SparklineStyle {
    /// Color of the data line.
    pub line_color: Color,
    /// Semi-transparent fill below the line.
    pub fill_color: Color,
    /// Line thickness in pixels.
    pub line_width: f32,
    /// Whether to render horizontal grid lines.
    pub show_grid: bool,
    /// Number of horizontal grid lines (only if `show_grid` is true).
    pub grid_lines: u32,
}

impl Default for SparklineStyle {
    fn default() -> Self {
        Self {
            line_color: Color::from_rgb(0.30, 0.70, 1.0),
            fill_color: Color::from_rgba(0.30, 0.70, 1.0, 0.15),
            line_width: 1.5,
            show_grid: true,
            grid_lines: 3,
        }
    }
}

/// A compact line chart that auto-scales to its data.
pub struct Sparkline {
    data: Vec<f64>,
    style: SparklineStyle,
    cache: canvas::Cache,
}

impl Sparkline {
    pub fn new(style: SparklineStyle) -> Self {
        Self {
            data: Vec::new(),
            style,
            cache: canvas::Cache::new(),
        }
    }

    /// Replaces the data buffer and triggers a redraw.
    pub fn set_data(&mut self, data: &[f64]) {
        self.data = data.to_vec();
        self.cache.clear();
    }

    /// Replaces the visual style and triggers a redraw.
    pub fn set_style(&mut self, style: SparklineStyle) {
        self.style = style;
        self.cache.clear();
    }

    /// Computes (min, max) with a small padding so flat lines don't collapse.
    fn y_range(&self) -> (f64, f64) {
        if self.data.is_empty() {
            return (0.0, 1.0);
        }

        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for &v in &self.data {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }

        // Prevent zero-height range.
        if (max - min).abs() < 1e-9 {
            let pad = if min.abs() < 1e-9 { 1.0 } else { min.abs() * 0.1 };
            min -= pad;
            max += pad;
        }

        // Add 5% vertical padding.
        let padding = (max - min) * 0.05;
        (min - padding, max + padding)
    }
}

impl Default for Sparkline {
    fn default() -> Self {
        Self::new(SparklineStyle::default())
    }
}

impl<Message> canvas::Program<Message, cosmic::Theme, cosmic::Renderer> for Sparkline {
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
            let size = bounds.size();
            let (y_min, y_max) = self.y_range();
            let y_span = y_max - y_min;

            // Grid lines.
            if self.style.show_grid && self.style.grid_lines > 0 {
                draw_grid(frame, size, self.style.grid_lines);
            }

            if self.data.len() < 2 {
                // Single point: draw a dot at center-y.
                if self.data.len() == 1 {
                    let y = map_y(self.data[0], y_min, y_span, size.height);
                    let dot = Path::circle(Point::new(size.width / 2.0, y), 3.0);
                    frame.fill(&dot, self.style.line_color);
                }
                return;
            }

            let points = compute_points(&self.data, y_min, y_span, size);

            // Semi-transparent fill below the line.
            let fill_path = Path::new(|b| {
                b.move_to(Point::new(points[0].x, size.height));
                for &p in &points {
                    b.line_to(p);
                }
                b.line_to(Point::new(points.last().unwrap().x, size.height));
                b.close();
            });
            frame.fill(&fill_path, self.style.fill_color);

            // Data line.
            let line = Path::new(|b| {
                b.move_to(points[0]);
                for &p in &points[1..] {
                    b.line_to(p);
                }
            });
            frame.stroke(
                &line,
                Stroke::default()
                    .with_width(self.style.line_width)
                    .with_color(self.style.line_color),
            );
        });

        vec![geometry]
    }
}

/// Maps a data value to a Y pixel coordinate (top = max, bottom = min).
fn map_y(value: f64, y_min: f64, y_span: f64, height: f32) -> f32 {
    let ratio = (value - y_min) / y_span;
    height * (1.0 - ratio as f32)
}

/// Converts data points to pixel coordinates evenly spaced along X.
fn compute_points(data: &[f64], y_min: f64, y_span: f64, size: Size) -> Vec<Point> {
    let n = data.len();
    let x_step = size.width / (n - 1) as f32;

    data.iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = i as f32 * x_step;
            let y = map_y(v, y_min, y_span, size.height);
            Point::new(x, y)
        })
        .collect()
}

/// Draws subtle horizontal grid lines.
fn draw_grid(frame: &mut Frame, size: Size, lines: u32) {
    for i in 1..=lines {
        let y = size.height * i as f32 / (lines + 1) as f32;
        let grid_line = Path::line(Point::new(0.0, y), Point::new(size.width, y));
        frame.stroke(
            &grid_line,
            Stroke::default()
                .with_width(1.0)
                .with_color(GRID_COLOR),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn y_range_empty() {
        let s = Sparkline::default();
        let (min, max) = s.y_range();
        assert!((min - 0.0).abs() < 1e-5);
        assert!((max - 1.0).abs() < 1e-5);
    }

    #[test]
    fn y_range_single_value() {
        let mut s = Sparkline::default();
        s.set_data(&[50.0]);
        let (min, max) = s.y_range();
        // Should have padding around the single value.
        assert!(min < 50.0);
        assert!(max > 50.0);
    }

    #[test]
    fn y_range_flat_line() {
        let mut s = Sparkline::default();
        s.set_data(&[10.0, 10.0, 10.0]);
        let (min, max) = s.y_range();
        assert!(min < 10.0);
        assert!(max > 10.0);
    }

    #[test]
    fn y_range_flat_zero() {
        let mut s = Sparkline::default();
        s.set_data(&[0.0, 0.0, 0.0]);
        let (min, max) = s.y_range();
        assert!(min < 0.0);
        assert!(max > 0.0);
    }

    #[test]
    fn y_range_normal() {
        let mut s = Sparkline::default();
        s.set_data(&[10.0, 20.0, 30.0]);
        let (min, max) = s.y_range();
        // With 5% padding.
        assert!(min < 10.0);
        assert!(max > 30.0);
    }

    #[test]
    fn compute_points_evenly_spaced() {
        let data = [0.0, 50.0, 100.0];
        let size = Size::new(100.0, 100.0);
        let (y_min, y_span) = (0.0, 100.0);
        let pts = compute_points(&data, y_min, y_span, size);

        assert_eq!(pts.len(), 3);
        assert!((pts[0].x - 0.0).abs() < 1e-5);
        assert!((pts[1].x - 50.0).abs() < 1e-5);
        assert!((pts[2].x - 100.0).abs() < 1e-5);
    }

    #[test]
    fn map_y_extremes() {
        // min maps to bottom (height), max maps to top (0).
        assert!((map_y(0.0, 0.0, 100.0, 200.0) - 200.0).abs() < 1e-5);
        assert!((map_y(100.0, 0.0, 100.0, 200.0) - 0.0).abs() < 1e-5);
        assert!((map_y(50.0, 0.0, 100.0, 200.0) - 100.0).abs() < 1e-5);
    }
}
