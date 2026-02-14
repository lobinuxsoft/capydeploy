use cosmic::iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use cosmic::iced::{alignment, mouse, Color, Point, Rectangle, Size};

use crate::colors::GRID_COLOR;
use crate::sparkline::SparklineStyle;

/// Vertical gap between stacked sparkline sections.
const SECTION_GAP: f32 = 8.0;
/// Height of the label text area above each sparkline.
const LABEL_HEIGHT: f32 = 16.0;
/// Label font size.
const LABEL_FONT_SIZE: f32 = 12.0;
/// Label text color (muted white).
const LABEL_COLOR: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.5);

/// A single sparkline dataset with its label and visual style.
struct SparklineEntry {
    label: String,
    data: Vec<f64>,
    style: SparklineStyle,
}

/// Multiple sparklines rendered in a single canvas widget.
///
/// Works around iced bug #3040 where multiple `canvas::Canvas` widgets
/// in the same view cause geometry not to render. By consolidating all
/// sparklines into one canvas, we stay within the single-canvas-per-view
/// limit that actually works.
pub struct CombinedSparklines {
    entries: Vec<SparklineEntry>,
    cache: canvas::Cache,
}

impl CombinedSparklines {
    /// Creates a new combined sparklines widget with the given labels and styles.
    ///
    /// Each `(label, style)` pair defines one sparkline section stacked vertically.
    pub fn new(configs: Vec<(String, SparklineStyle)>) -> Self {
        let entries = configs
            .into_iter()
            .map(|(label, style)| SparklineEntry {
                label,
                data: Vec::new(),
                style,
            })
            .collect();

        Self {
            entries,
            cache: canvas::Cache::new(),
        }
    }

    /// Updates the data for a specific sparkline by index and clears the cache.
    pub fn set_data(&mut self, index: usize, data: &[f64]) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.data = data.to_vec();
            self.cache.clear();
        }
    }

    /// Returns the number of sparkline sections.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if there are no sparkline sections.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Computes the recommended total height for the widget.
    ///
    /// Each section gets `section_height` pixels for the chart plus
    /// `LABEL_HEIGHT` for the label, with `SECTION_GAP` between sections.
    pub fn recommended_height(&self, section_chart_height: f32) -> f32 {
        let n = self.entries.len() as f32;
        if n == 0.0 {
            return 0.0;
        }
        n * (LABEL_HEIGHT + section_chart_height) + (n - 1.0) * SECTION_GAP
    }
}

impl<Message> canvas::Program<Message, cosmic::Theme, cosmic::Renderer> for CombinedSparklines {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if self.entries.is_empty() {
            return vec![];
        }

        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let total_height = bounds.height;
            let n = self.entries.len() as f32;
            let total_gaps = (n - 1.0) * SECTION_GAP;
            let total_labels = n * LABEL_HEIGHT;
            let chart_height = ((total_height - total_gaps - total_labels) / n).max(20.0);

            for (i, entry) in self.entries.iter().enumerate() {
                let y_offset =
                    i as f32 * (LABEL_HEIGHT + chart_height + SECTION_GAP);

                // Draw label.
                frame.fill_text(Text {
                    content: entry.label.clone(),
                    position: Point::new(0.0, y_offset + LABEL_FONT_SIZE * 0.8),
                    color: LABEL_COLOR,
                    size: LABEL_FONT_SIZE.into(),
                    horizontal_alignment: alignment::Horizontal::Left,
                    vertical_alignment: alignment::Vertical::Center,
                    ..Text::default()
                });

                let chart_y = y_offset + LABEL_HEIGHT;
                let chart_size = Size::new(bounds.width, chart_height);

                // Draw chart content for this section.
                draw_sparkline_section(frame, entry, chart_y, chart_size);
            }
        });

        vec![geometry]
    }
}

/// Draws a single sparkline section (grid + fill + line) within the given bounds.
fn draw_sparkline_section(
    frame: &mut Frame,
    entry: &SparklineEntry,
    y_offset: f32,
    size: Size,
) {
    let grid_lines = if entry.style.show_grid {
        entry.style.grid_lines
    } else {
        0
    };

    // Grid lines.
    if grid_lines > 0 {
        for i in 1..=grid_lines {
            let y = y_offset + size.height * i as f32 / (grid_lines + 1) as f32;
            let line = Path::line(
                Point::new(0.0, y),
                Point::new(size.width, y),
            );
            frame.stroke(
                &line,
                Stroke::default().with_width(1.0).with_color(GRID_COLOR),
            );
        }
    }

    if entry.data.is_empty() {
        return;
    }

    let (y_min, y_max) = y_range(&entry.data);
    let y_span = y_max - y_min;

    if entry.data.len() == 1 {
        // Single point: dot at center.
        let y = map_y(entry.data[0], y_min, y_span, size.height) + y_offset;
        let dot = Path::circle(Point::new(size.width / 2.0, y), 3.0);
        frame.fill(&dot, entry.style.line_color);
        return;
    }

    let points = compute_points(&entry.data, y_min, y_span, y_offset, size);

    // Semi-transparent fill below the line.
    let bottom = y_offset + size.height;
    let fill_path = Path::new(|b| {
        b.move_to(Point::new(points[0].x, bottom));
        for &p in &points {
            b.line_to(p);
        }
        b.line_to(Point::new(points.last().unwrap().x, bottom));
        b.close();
    });
    frame.fill(&fill_path, entry.style.fill_color);

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
            .with_width(entry.style.line_width)
            .with_color(entry.style.line_color),
    );
}

/// Computes (min, max) with padding so flat lines don't collapse.
fn y_range(data: &[f64]) -> (f64, f64) {
    if data.is_empty() {
        return (0.0, 1.0);
    }

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in data {
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }

    if (max - min).abs() < 1e-9 {
        let pad = if min.abs() < 1e-9 { 1.0 } else { min.abs() * 0.1 };
        min -= pad;
        max += pad;
    }

    let padding = (max - min) * 0.05;
    (min - padding, max + padding)
}

/// Maps a data value to a Y pixel coordinate (top = max, bottom = min).
fn map_y(value: f64, y_min: f64, y_span: f64, height: f32) -> f32 {
    let ratio = (value - y_min) / y_span;
    height * (1.0 - ratio as f32)
}

/// Converts data points to pixel coordinates, offset vertically.
fn compute_points(
    data: &[f64],
    y_min: f64,
    y_span: f64,
    y_offset: f32,
    size: Size,
) -> Vec<Point> {
    let n = data.len();
    let x_step = size.width / (n - 1) as f32;

    data.iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = i as f32 * x_step;
            let y = map_y(v, y_min, y_span, size.height) + y_offset;
            Point::new(x, y)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_entries() {
        let cs = CombinedSparklines::new(vec![
            ("CPU".into(), SparklineStyle::default()),
            ("GPU".into(), SparklineStyle::default()),
        ]);
        assert_eq!(cs.len(), 2);
        assert!(!cs.is_empty());
    }

    #[test]
    fn set_data_updates_entry() {
        let mut cs = CombinedSparklines::new(vec![
            ("CPU".into(), SparklineStyle::default()),
        ]);
        cs.set_data(0, &[10.0, 20.0, 30.0]);
        assert_eq!(cs.entries[0].data.len(), 3);
    }

    #[test]
    fn set_data_out_of_bounds_is_noop() {
        let mut cs = CombinedSparklines::new(vec![
            ("CPU".into(), SparklineStyle::default()),
        ]);
        cs.set_data(5, &[10.0]); // should not panic
    }

    #[test]
    fn recommended_height_calculation() {
        let cs = CombinedSparklines::new(vec![
            ("A".into(), SparklineStyle::default()),
            ("B".into(), SparklineStyle::default()),
            ("C".into(), SparklineStyle::default()),
        ]);
        let h = cs.recommended_height(60.0);
        // 3 * (16 + 60) + 2 * 8 = 228 + 16 = 244
        assert!((h - 244.0).abs() < 1e-5);
    }

    #[test]
    fn y_range_empty() {
        let (min, max) = y_range(&[]);
        assert!((min - 0.0).abs() < 1e-5);
        assert!((max - 1.0).abs() < 1e-5);
    }

    #[test]
    fn y_range_flat() {
        let (min, max) = y_range(&[42.0, 42.0, 42.0]);
        assert!(min < 42.0);
        assert!(max > 42.0);
    }

    #[test]
    fn y_range_normal() {
        let (min, max) = y_range(&[10.0, 50.0]);
        assert!(min < 10.0);
        assert!(max > 50.0);
    }

    #[test]
    fn map_y_extremes() {
        assert!((map_y(0.0, 0.0, 100.0, 200.0) - 200.0).abs() < 1e-5);
        assert!((map_y(100.0, 0.0, 100.0, 200.0) - 0.0).abs() < 1e-5);
    }
}
