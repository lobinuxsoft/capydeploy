//! Telemetry dashboard — progress-bar gauges, sparklines, and system info.

use cosmic::iced::widget::horizontal_space;
use cosmic::iced::{Color, Length};
use cosmic::widget::{self, canvas, container};
use cosmic::Element;

use capydeploy_hub_telemetry::TelemetryHub;

use crate::message::Message;
use crate::theme;

use super::TelemetryWidgets;

// Color thresholds for usage and temperature.
const GREEN: Color = Color::from_rgb(0.18, 0.80, 0.44);
const YELLOW: Color = Color::from_rgb(0.90, 0.75, 0.10);
const RED: Color = Color::from_rgb(0.90, 0.25, 0.20);

/// Renders the telemetry dashboard.
pub fn view<'a>(
    hub: &'a TelemetryHub,
    agent_id: Option<&str>,
    widgets: &'a TelemetryWidgets,
) -> Element<'a, Message> {
    let mut content = widget::column().spacing(12);

    content = content.push(widget::text::title3("Telemetry"));

    let agent = agent_id.and_then(|id| hub.get_agent(id));

    let has_data = agent.is_some_and(|a| a.latest().is_some());

    if !has_data {
        content = content.push(
            widget::text("Waiting for telemetry data...").class(theme::MUTED_TEXT),
        );
        return content.into();
    }

    let agent = agent.unwrap();
    let latest = agent.latest().unwrap();

    // -- Metric cards (2x2 grid via flex_row) --
    let mut cards: Vec<Element<'a, Message>> = Vec::new();

    if let Some(cpu) = &latest.cpu {
        cards.push(metric_card(
            "CPU",
            &format!("{:.1}%", cpu.usage_percent),
            cpu.usage_percent / 100.0,
            usage_color(cpu.usage_percent),
        ));
    }

    if let Some(gpu) = &latest.gpu {
        cards.push(metric_card(
            "GPU",
            &format!("{:.1}%", gpu.usage_percent),
            gpu.usage_percent / 100.0,
            usage_color(gpu.usage_percent),
        ));
    }

    if let Some(cpu) = &latest.cpu {
        cards.push(metric_card(
            "Temp",
            &format!("{:.1}°C", cpu.temp_celsius),
            cpu.temp_celsius / 100.0,
            temp_color(cpu.temp_celsius),
        ));
    }

    if let Some(mem) = &latest.memory {
        cards.push(metric_card(
            "Mem",
            &format!("{:.1}%", mem.usage_percent),
            mem.usage_percent / 100.0,
            usage_color(mem.usage_percent),
        ));
    }

    if !cards.is_empty() {
        content = content.push(
            widget::flex_row(cards)
                .column_spacing(12)
                .row_spacing(12),
        );
    }

    // -- Sparklines (canvas) --
    content = content.push(sparkline_card("CPU Usage", &widgets.cpu_sparkline));
    content = content.push(sparkline_card("GPU Usage", &widgets.gpu_sparkline));
    content = content.push(sparkline_card("Memory Usage", &widgets.mem_sparkline));

    // -- System info card --
    if let Some(latest) = agent.latest() {
        let mut info_col = widget::column().spacing(4);

        if let Some(cpu) = &latest.cpu {
            info_col = info_col.push(info_entry("CPU Freq", &format_freq(cpu.freq_m_hz)));
        }

        if let Some(gpu) = &latest.gpu {
            info_col = info_col.push(info_entry("GPU Freq", &format_freq(gpu.freq_m_hz)));
            if gpu.vram_total_bytes > 0 {
                let used_mb = gpu.vram_used_bytes as f64 / 1_048_576.0;
                let total_mb = gpu.vram_total_bytes as f64 / 1_048_576.0;
                info_col = info_col.push(info_entry(
                    "VRAM",
                    &format!("{:.0} / {:.0} MB", used_mb, total_mb),
                ));
            }
        }

        if let Some(mem) = &latest.memory {
            let total_gb = mem.total_bytes as f64 / 1_073_741_824.0;
            let avail_gb = mem.available_bytes as f64 / 1_073_741_824.0;
            let used_gb = total_gb - avail_gb;
            info_col = info_col.push(info_entry(
                "RAM",
                &format!("{:.1} / {:.1} GB", used_gb, total_gb),
            ));
            if mem.swap_total_bytes > 0 {
                let swap_total = mem.swap_total_bytes as f64 / 1_073_741_824.0;
                let swap_free = mem.swap_free_bytes as f64 / 1_073_741_824.0;
                let swap_used = swap_total - swap_free;
                info_col = info_col.push(info_entry(
                    "Swap",
                    &format!("{:.1} / {:.1} GB", swap_used, swap_total),
                ));
            }
        }

        if let Some(bat) = &latest.battery {
            info_col = info_col.push(info_entry(
                "Battery",
                &format!("{}% ({})", bat.capacity, bat.status),
            ));
        }

        if let Some(power) = &latest.power {
            if power.tdp_watts > 0.0 {
                info_col = info_col.push(info_entry(
                    "Power",
                    &format!("{:.1}W / {:.1}W TDP", power.power_watts, power.tdp_watts),
                ));
            } else if power.power_watts > 0.0 {
                info_col = info_col.push(info_entry(
                    "Power",
                    &format!("{:.1}W", power.power_watts),
                ));
            }
        }

        if let Some(fan) = &latest.fan {
            info_col = info_col.push(info_entry("Fan", &format!("{} RPM", fan.rpm)));
        }

        if let Some(steam) = &latest.steam {
            let status = if steam.gaming_mode {
                "Gaming Mode"
            } else if steam.running {
                "Running"
            } else {
                "Not running"
            };
            info_col = info_col.push(info_entry("Steam", status));
        }

        content = content.push(
            container(info_col.padding(12))
                .width(Length::Fill)
                .class(cosmic::theme::Container::Custom(Box::new(
                    theme::canvas_bg,
                ))),
        );
    }

    widget::scrollable(content).into()
}

// ---------------------------------------------------------------------------
// Metric card (progress bar based — guaranteed to render)
// ---------------------------------------------------------------------------

/// Builds a metric card with big value text, a colored progress bar, and label.
fn metric_card(
    label: &str,
    value_text: &str,
    ratio: f64,
    color: Color,
) -> Element<'static, Message> {
    let clamped = (ratio as f32).clamp(0.0, 1.0);

    let col = widget::column()
        .push(
            widget::text::title3(value_text.to_string())
                .class(color),
        )
        .push(widget::progress_bar(0.0..=1.0, clamped).height(8))
        .push(
            widget::text::caption(label.to_string())
                .class(theme::MUTED_TEXT),
        )
        .spacing(6)
        .align_x(cosmic::iced::Alignment::Center);

    container(col.padding(12))
        .width(Length::Fixed(160.0))
        .class(cosmic::theme::Container::Custom(Box::new(
            theme::canvas_bg,
        )))
        .into()
}

// ---------------------------------------------------------------------------
// Sparklines (canvas — these render correctly)
// ---------------------------------------------------------------------------

/// Wraps a sparkline widget in a labeled card container.
fn sparkline_card<'a>(
    label: &'a str,
    sparkline: &'a capydeploy_hub_widgets::Sparkline,
) -> Element<'a, Message> {
    let header = widget::text::caption(label).class(theme::MUTED_TEXT);
    let canvas_widget = canvas::Canvas::new(sparkline)
        .width(Length::Fill)
        .height(Length::Fixed(80.0));

    container(
        widget::column()
            .push(header)
            .push(canvas_widget)
            .spacing(4)
            .padding(12),
    )
    .width(Length::Fill)
    .class(cosmic::theme::Container::Custom(Box::new(
        theme::canvas_bg,
    )))
    .into()
}

// ---------------------------------------------------------------------------
// Info helpers
// ---------------------------------------------------------------------------

/// A label: value row for system info entries.
fn info_entry(label: &str, value: &str) -> Element<'static, Message> {
    widget::row()
        .push(widget::text::caption(label.to_string()).class(theme::MUTED_TEXT))
        .push(horizontal_space())
        .push(widget::text::caption(value.to_string()))
        .into()
}

fn format_freq(mhz: f64) -> String {
    if mhz >= 1000.0 {
        format!("{:.2} GHz", mhz / 1000.0)
    } else {
        format!("{:.0} MHz", mhz)
    }
}

fn usage_color(percent: f64) -> Color {
    if percent < 60.0 {
        GREEN
    } else if percent <= 80.0 {
        YELLOW
    } else {
        RED
    }
}

fn temp_color(celsius: f64) -> Color {
    if celsius < 0.0 {
        theme::MUTED_TEXT
    } else if celsius < 60.0 {
        GREEN
    } else if celsius <= 80.0 {
        YELLOW
    } else {
        RED
    }
}
