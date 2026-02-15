//! Telemetry dashboard — progress-bar gauges and system info.

use cosmic::iced::widget::horizontal_space;
use cosmic::iced::{Color, Length};
use cosmic::widget::{self, container};
use cosmic::Element;

use capydeploy_hub_telemetry::TelemetryHub;

use crate::message::Message;
use crate::theme;

// Color thresholds for usage and temperature.
const GREEN: Color = Color::from_rgb(0.18, 0.80, 0.44);
const YELLOW: Color = Color::from_rgb(0.90, 0.75, 0.10);
const RED: Color = Color::from_rgb(0.90, 0.25, 0.20);

/// Renders the telemetry dashboard.
pub fn view<'a>(hub: &'a TelemetryHub, agent_id: Option<&str>) -> Element<'a, Message> {
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

    // -- Metric cards (flex_row grid) --
    let mut cards: Vec<Element<'a, Message>> = Vec::new();

    // CPU usage.
    if let Some(cpu) = &latest.cpu {
        cards.push(metric_card(
            "CPU",
            &format!("{:.1}%", cpu.usage_percent),
            cpu.usage_percent / 100.0,
            usage_color(cpu.usage_percent),
        ));
    }

    // GPU usage.
    if let Some(gpu) = &latest.gpu {
        cards.push(metric_card(
            "GPU",
            &format!("{:.1}%", gpu.usage_percent),
            gpu.usage_percent / 100.0,
            usage_color(gpu.usage_percent),
        ));
    }

    // CPU temperature.
    if let Some(cpu) = &latest.cpu {
        cards.push(metric_card(
            "CPU Temp",
            &format!("{:.0}°C", cpu.temp_celsius),
            cpu.temp_celsius / 100.0,
            temp_color(cpu.temp_celsius),
        ));
    }

    // GPU temperature.
    if let Some(gpu) = &latest.gpu {
        cards.push(metric_card(
            "GPU Temp",
            &format!("{:.0}°C", gpu.temp_celsius),
            gpu.temp_celsius / 100.0,
            temp_color(gpu.temp_celsius),
        ));
    }

    // RAM usage.
    if let Some(mem) = &latest.memory {
        let total_gb = mem.total_bytes as f64 / 1_073_741_824.0;
        let avail_gb = mem.available_bytes as f64 / 1_073_741_824.0;
        let used_gb = total_gb - avail_gb;
        cards.push(metric_card(
            "RAM",
            &format!("{:.1}/{:.1} GB", used_gb, total_gb),
            mem.usage_percent / 100.0,
            usage_color(mem.usage_percent),
        ));
    }

    // VRAM.
    if let Some(gpu) = &latest.gpu
        && gpu.vram_total_bytes > 0
    {
        let used_mb = gpu.vram_used_bytes as f64 / 1_048_576.0;
        let total_mb = gpu.vram_total_bytes as f64 / 1_048_576.0;
        let ratio = used_mb / total_mb;
        let pct = ratio * 100.0;
        cards.push(metric_card(
            "VRAM",
            &format!("{:.0}/{:.0} MB", used_mb, total_mb),
            ratio,
            usage_color(pct),
        ));
    }

    // Battery.
    if let Some(bat) = &latest.battery {
        let cap = bat.capacity as f64;
        let color = if cap > 50.0 {
            GREEN
        } else if cap > 20.0 {
            YELLOW
        } else {
            RED
        };
        cards.push(metric_card(
            "Battery",
            &format!("{}% ({})", bat.capacity, bat.status),
            cap / 100.0,
            color,
        ));
    }

    // Power / TDP.
    if let Some(power) = &latest.power {
        if power.tdp_watts > 0.0 {
            let ratio = power.power_watts / power.tdp_watts;
            cards.push(metric_card(
                "Power",
                &format!("{:.1}W / {:.1}W", power.power_watts, power.tdp_watts),
                ratio,
                usage_color(ratio * 100.0),
            ));
        } else if power.power_watts > 0.0 {
            // No TDP reference — show bar at fixed 50% just as visual indicator.
            cards.push(metric_card(
                "Power",
                &format!("{:.1}W", power.power_watts),
                0.5,
                GREEN,
            ));
        }
    }

    // Fan RPM (no natural max — show as text-only card).
    if let Some(fan) = &latest.fan {
        cards.push(text_card("Fan", &format!("{} RPM", fan.rpm)));
    }

    if !cards.is_empty() {
        content = content.push(
            widget::flex_row(cards)
                .column_spacing(12)
                .row_spacing(12),
        );
    }

    // -- System info (frequencies, swap, Steam status) --
    let mut info_entries: Vec<Element<'_, Message>> = Vec::new();

    if let Some(cpu) = &latest.cpu {
        info_entries.push(info_entry("CPU Freq", &format_freq(cpu.freq_m_hz)));
    }

    if let Some(gpu) = &latest.gpu {
        info_entries.push(info_entry("GPU Freq", &format_freq(gpu.freq_m_hz)));
        if gpu.mem_freq_m_hz > 0.0 {
            info_entries.push(info_entry(
                "VRAM Freq",
                &format_freq(gpu.mem_freq_m_hz),
            ));
        }
    }

    if let Some(mem) = &latest.memory
        && mem.swap_total_bytes > 0
    {
        let swap_total = mem.swap_total_bytes as f64 / 1_073_741_824.0;
        let swap_free = mem.swap_free_bytes as f64 / 1_073_741_824.0;
        let swap_used = swap_total - swap_free;
        info_entries.push(info_entry(
            "Swap",
            &format!("{:.1} / {:.1} GB", swap_used, swap_total),
        ));
    }

    if let Some(steam) = &latest.steam {
        let status = if steam.gaming_mode {
            "Gaming Mode"
        } else if steam.running {
            "Running"
        } else {
            "Not running"
        };
        info_entries.push(info_entry("Steam", status));
    }

    if !info_entries.is_empty() {
        let mut info_col = widget::column().spacing(4);
        for entry in info_entries {
            info_col = info_col.push(entry);
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
// Metric card (progress bar based)
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

/// Builds a text-only card (no progress bar) for metrics without a natural range.
fn text_card(label: &str, value_text: &str) -> Element<'static, Message> {
    let col = widget::column()
        .push(widget::text::title3(value_text.to_string()))
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
