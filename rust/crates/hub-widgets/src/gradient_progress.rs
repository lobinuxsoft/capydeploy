use cosmic::iced::widget::canvas::{self, Fill, Path, Text};
use cosmic::iced::{alignment, mouse, Color, Point, Rectangle, Size};

use crate::colors::{GREEN, RED, TRACK_GRAY, YELLOW};

/// Corner radius for the progress bar rectangles.
const CORNER_RADIUS: f32 = 4.0;

/// Optional label displayed on top of the progress bar.
#[derive(Debug, Clone, Default)]
pub enum ProgressLabel {
    #[default]
    None,
    /// Shows `"{value}%"`.
    Percentage,
    /// Shows `"{current} / {total}"` (e.g., "45.2 MB / 120 MB").
    Transfer { current: String, total: String },
}

/// A horizontal progress bar with a green → yellow → red linear gradient fill.
pub struct GradientProgress {
    value: f32,
    label: ProgressLabel,
    cache: canvas::Cache,
}

impl GradientProgress {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            label: ProgressLabel::None,
            cache: canvas::Cache::new(),
        }
    }

    /// Sets the progress value (clamped to `0.0..=1.0`) and clears the cache.
    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(0.0, 1.0);
        self.cache.clear();
    }

    /// Changes the label overlay.
    pub fn set_label(&mut self, label: ProgressLabel) {
        self.label = label;
        self.cache.clear();
    }

    fn label_text(&self) -> Option<String> {
        match &self.label {
            ProgressLabel::None => None,
            ProgressLabel::Percentage => Some(format!("{:.0}%", self.value * 100.0)),
            ProgressLabel::Transfer { current, total } => Some(format!("{current} / {total}")),
        }
    }
}

impl Default for GradientProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> canvas::Program<Message, cosmic::Theme, cosmic::Renderer> for GradientProgress {
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

            // Track background.
            let track = Path::rounded_rectangle(Point::ORIGIN, size, CORNER_RADIUS.into());
            frame.fill(&track, TRACK_GRAY);

            // Filled portion with gradient.
            if self.value > 0.0 {
                let fill_width = size.width * self.value;
                let fill_size = Size::new(fill_width, size.height);

                let gradient = canvas::Gradient::Linear(
                    canvas::gradient::Linear::new(
                        Point::ORIGIN,
                        Point::new(size.width, 0.0),
                    )
                    .add_stop(0.0, GREEN)
                    .add_stop(0.6, YELLOW)
                    .add_stop(1.0, RED),
                );

                let fill_rect =
                    Path::rounded_rectangle(Point::ORIGIN, fill_size, CORNER_RADIUS.into());
                frame.fill(
                    &fill_rect,
                    Fill {
                        style: canvas::fill::Style::Gradient(gradient),
                        ..Fill::default()
                    },
                );
            }

            // Text overlay.
            if let Some(text) = self.label_text() {
                let center = Point::new(size.width / 2.0, size.height / 2.0);
                let font_size = (size.height * 0.55).min(14.0);
                frame.fill_text(Text {
                    content: text,
                    position: center,
                    color: Color::WHITE,
                    size: font_size.into(),
                    horizontal_alignment: alignment::Horizontal::Center,
                    vertical_alignment: alignment::Vertical::Center,
                    ..Text::default()
                });
            }
        });

        vec![geometry]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_clamps() {
        let mut p = GradientProgress::new();
        p.set_value(1.5);
        assert!((p.value - 1.0).abs() < 1e-5);
        p.set_value(-0.5);
        assert!((p.value - 0.0).abs() < 1e-5);
    }

    #[test]
    fn label_text_none() {
        let p = GradientProgress::new();
        assert!(p.label_text().is_none());
    }

    #[test]
    fn label_text_percentage() {
        let mut p = GradientProgress::new();
        p.set_value(0.65);
        p.set_label(ProgressLabel::Percentage);
        assert_eq!(p.label_text().unwrap(), "65%");
    }

    #[test]
    fn label_text_transfer() {
        let mut p = GradientProgress::new();
        p.set_label(ProgressLabel::Transfer {
            current: "45.2 MB".into(),
            total: "120 MB".into(),
        });
        assert_eq!(p.label_text().unwrap(), "45.2 MB / 120 MB");
    }
}
