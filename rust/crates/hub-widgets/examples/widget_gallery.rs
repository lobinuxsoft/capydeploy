//! Interactive visual test for all hub-widgets: gauges, progress bars, and sparklines.
//!
//! Run with:
//! ```
//! cargo run --release --example widget_gallery -p capydeploy-hub-widgets --features example
//! ```
//!
//! Note: debug builds are very slow due to unoptimized wgpu. Always use `--release`.

use std::f64::consts::PI;

use cosmic::app::Core;
use cosmic::iced::widget::canvas;
use cosmic::iced::widget::container as iced_container;
use cosmic::iced::Length;
use cosmic::widget::{self, container};
use cosmic::{Application, Element};

use capydeploy_hub_widgets::{
    GaugeThresholds, GradientProgress, ProgressLabel, Sparkline, SparklineStyle, TelemetryGauge,
};

fn main() -> cosmic::iced::Result {
    let settings = cosmic::app::Settings::default().size(cosmic::iced::Size::new(900.0, 850.0));
    cosmic::app::run::<Gallery>(settings, ())
}

struct Gallery {
    core: Core,
    // Gauges
    cpu_gauge: TelemetryGauge,
    gpu_gauge: TelemetryGauge,
    temp_gauge: TelemetryGauge,
    cpu_value: f64,
    gpu_value: f64,
    temp_value: f64,
    // Progress
    progress: GradientProgress,
    progress_value: f64,
    label_mode: Option<usize>,
    label_options: Vec<String>,
    // Sparkline
    sparkline: Sparkline,
    data_mode: Option<usize>,
    data_options: Vec<String>,
    point_count: f64,
    show_grid: bool,
    line_width: f64,
    random_seed: u64,
}

#[derive(Debug, Clone)]
enum Message {
    CpuChanged(f64),
    GpuChanged(f64),
    TempChanged(f64),
    ProgressChanged(f64),
    LabelMode(usize),
    DataMode(usize),
    PointCount(f64),
    ToggleGrid(bool),
    LineWidth(f64),
    Randomize,
}

impl Gallery {
    fn regenerate_sparkline_data(&mut self) {
        let count = self.point_count as usize;
        let data = match self.data_mode.unwrap_or(0) {
            1 => generate_random_walk(count, self.random_seed),
            2 => generate_sawtooth(count, self.random_seed),
            _ => generate_sin_wave(count, self.random_seed),
        };
        self.sparkline.set_data(&data);
    }

    fn update_sparkline_style(&mut self) {
        let mut style = SparklineStyle::default();
        style.show_grid = self.show_grid;
        style.line_width = self.line_width as f32;
        self.sparkline.set_style(style);
    }

    fn update_progress_label(&mut self) {
        let label = match self.label_mode.unwrap_or(0) {
            1 => ProgressLabel::Percentage,
            2 => ProgressLabel::Transfer {
                current: format!("{:.1} MB", self.progress_value * 1.2),
                total: "120 MB".into(),
            },
            _ => ProgressLabel::None,
        };
        self.progress.set_label(label);
    }
}

impl Application for Gallery {
    type Executor = cosmic::executor::Default;
    type Message = Message;
    type Flags = ();

    const APP_ID: &'static str = "com.capydeploy.widget-gallery";

    fn init(mut core: Core, _flags: ()) -> (Self, cosmic::app::Task<Message>) {
        // Disable COSMIC CSD header — KDE/other WMs provide their own buttons.
        core.window.show_headerbar = false;

        let cpu_value = 25.0;
        let gpu_value = 65.0;
        let temp_value = 90.0;

        let mut cpu_gauge = TelemetryGauge::new("CPU", "%", 0.0, 100.0);
        cpu_gauge.set_value(cpu_value);

        let mut gpu_gauge = TelemetryGauge::new("GPU", "%", 0.0, 100.0);
        gpu_gauge.set_value(gpu_value);

        let mut temp_gauge = TelemetryGauge::new("Temp", "°C", 0.0, 100.0)
            .with_thresholds(GaugeThresholds {
                warning: 0.6,
                critical: 0.8,
            });
        temp_gauge.set_value(temp_value);

        let progress_value = 50.0;
        let mut progress = GradientProgress::new();
        progress.set_value(progress_value as f32 / 100.0);

        let point_count = 50.0;
        let sin_data = generate_sin_wave(point_count as usize, 0);
        let mut sparkline = Sparkline::new(SparklineStyle::default());
        sparkline.set_data(&sin_data);

        let app = Self {
            core,
            cpu_gauge,
            gpu_gauge,
            temp_gauge,
            cpu_value,
            gpu_value,
            temp_value,
            progress,
            progress_value,
            label_mode: Some(0),
            label_options: vec![
                "None".into(),
                "Percentage".into(),
                "Transfer".into(),
            ],
            sparkline,
            data_mode: Some(0),
            data_options: vec![
                "Sin wave".into(),
                "Random walk".into(),
                "Sawtooth".into(),
            ],
            point_count,
            show_grid: true,
            line_width: 1.5,
            random_seed: 0,
        };
        (app, cosmic::app::Task::none())
    }

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        match message {
            Message::CpuChanged(v) => {
                self.cpu_value = v;
                self.cpu_gauge.set_value(v);
            }
            Message::GpuChanged(v) => {
                self.gpu_value = v;
                self.gpu_gauge.set_value(v);
            }
            Message::TempChanged(v) => {
                self.temp_value = v;
                self.temp_gauge.set_value(v);
            }
            Message::ProgressChanged(v) => {
                self.progress_value = v;
                self.progress.set_value(v as f32 / 100.0);
                self.update_progress_label();
            }
            Message::LabelMode(idx) => {
                self.label_mode = Some(idx);
                self.update_progress_label();
            }
            Message::DataMode(idx) => {
                self.data_mode = Some(idx);
                self.regenerate_sparkline_data();
            }
            Message::PointCount(v) => {
                self.point_count = v.round();
                self.regenerate_sparkline_data();
            }
            Message::ToggleGrid(v) => {
                self.show_grid = v;
                self.update_sparkline_style();
            }
            Message::LineWidth(v) => {
                self.line_width = v;
                self.update_sparkline_style();
            }
            Message::Randomize => {
                self.random_seed += 1;
                self.regenerate_sparkline_data();
            }
        }
        cosmic::app::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let title = widget::text::title3("CapyDeploy Widget Gallery");

        // --- Telemetry Gauges ---
        let gauges_heading = widget::text::heading("Telemetry Gauges");

        let gauges_row = widget::row()
            .push(gauge_canvas(&self.cpu_gauge))
            .push(gauge_canvas(&self.gpu_gauge))
            .push(gauge_canvas(&self.temp_gauge))
            .spacing(20);

        let gauges_card = container(gauges_row)
            .padding(12)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Custom(Box::new(canvas_bg)));

        let gauge_controls = widget::column()
            .push(widget::settings::item(
                "CPU",
                widget::slider(0.0..=100.0, self.cpu_value, Message::CpuChanged),
            ))
            .push(widget::settings::item(
                "GPU",
                widget::slider(0.0..=100.0, self.gpu_value, Message::GpuChanged),
            ))
            .push(widget::settings::item(
                "Temp",
                widget::slider(0.0..=100.0, self.temp_value, Message::TempChanged),
            ))
            .spacing(4);

        let gauges_section = widget::column()
            .push(gauges_heading)
            .push(gauges_card)
            .push(gauge_controls)
            .spacing(12);

        // --- Progress Bar ---
        let progress_heading = widget::text::heading("Progress Bar");

        let progress_canvas = container(
            canvas::Canvas::new(&self.progress)
                .width(Length::Fill)
                .height(Length::Fixed(36.0)),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(canvas_bg)));

        let progress_controls = widget::column()
            .push(widget::settings::item(
                "Value",
                widget::slider(0.0..=100.0, self.progress_value, Message::ProgressChanged),
            ))
            .push(widget::settings::item(
                "Label",
                widget::dropdown(&self.label_options, self.label_mode, Message::LabelMode),
            ))
            .spacing(4);

        let progress_section = widget::column()
            .push(progress_heading)
            .push(progress_canvas)
            .push(progress_controls)
            .spacing(12);

        // --- Sparkline ---
        let sparkline_heading = widget::text::heading("Sparkline");

        let sparkline_canvas = container(
            canvas::Canvas::new(&self.sparkline)
                .width(Length::Fill)
                .height(Length::Fixed(120.0)),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(canvas_bg)));

        let sparkline_controls = widget::column()
            .push(widget::settings::item(
                "Data",
                widget::dropdown(&self.data_options, self.data_mode, Message::DataMode),
            ))
            .push(widget::settings::item(
                "Points",
                widget::slider(10.0..=200.0, self.point_count, Message::PointCount).step(1.0),
            ))
            .push(widget::settings::item(
                "Grid",
                widget::toggler(self.show_grid).on_toggle(Message::ToggleGrid),
            ))
            .push(widget::settings::item(
                "Line width",
                widget::slider(0.5..=5.0, self.line_width, Message::LineWidth).step(0.5),
            ))
            .push(
                widget::button::standard("Randomize").on_press(Message::Randomize),
            )
            .spacing(4);

        let sparkline_section = widget::column()
            .push(sparkline_heading)
            .push(sparkline_canvas)
            .push(sparkline_controls)
            .spacing(12);

        // --- Assemble ---
        let content = widget::column()
            .push(title)
            .push(gauges_section)
            .push(widget::divider::horizontal::default())
            .push(progress_section)
            .push(widget::divider::horizontal::default())
            .push(sparkline_section)
            .spacing(16)
            .padding(24);

        let scrollable = widget::scrollable(content);

        container(scrollable)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn gauge_canvas(
    program: &impl canvas::Program<Message, cosmic::Theme, cosmic::Renderer>,
) -> canvas::Canvas<
    &impl canvas::Program<Message, cosmic::Theme, cosmic::Renderer>,
    Message,
    cosmic::Theme,
    cosmic::Renderer,
> {
    canvas::Canvas::new(program)
        .width(Length::Fixed(180.0))
        .height(Length::Fixed(180.0))
}

/// Static dark background for canvas containers — avoids Z-fighting that
/// `Container::Card` causes (Card has hover effects that trigger redraws).
fn canvas_bg(_theme: &cosmic::Theme) -> iced_container::Style {
    iced_container::Style {
        background: Some(cosmic::iced::Background::Color(
            cosmic::iced::Color::from_rgb(0.10, 0.10, 0.12),
        )),
        border: cosmic::iced::Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// --- Data generators ---

fn generate_sin_wave(count: usize, seed: u64) -> Vec<f64> {
    let phase = seed as f64 * 0.7;
    (0..count)
        .map(|i| (i as f64 * PI / 12.0 + phase).sin() * 50.0 + 50.0)
        .collect()
}

fn generate_random_walk(count: usize, seed: u64) -> Vec<f64> {
    let mut value = 50.0;
    let mut data = Vec::with_capacity(count);
    for i in 0..count {
        data.push(value);
        let hash = (i as u64)
            .wrapping_add(seed.wrapping_mul(2654435761))
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let step = ((hash >> 33) as f64 / u32::MAX as f64) * 10.0 - 5.0;
        value = (value + step).clamp(0.0, 100.0);
    }
    data
}

fn generate_sawtooth(count: usize, seed: u64) -> Vec<f64> {
    let offset = (seed % 20) as usize;
    (0..count)
        .map(|i| ((i + offset) % 20) as f64 * 5.0)
        .collect()
}
