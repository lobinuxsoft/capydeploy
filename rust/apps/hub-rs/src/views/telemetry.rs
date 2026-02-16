//! Telemetry dashboard — grouped metric cards with colored progress bars.

use cosmic::iced::widget::horizontal_space;
use cosmic::iced::{Color, Length};
use cosmic::widget::{self, container};
use cosmic::Element;

use capydeploy_hub_telemetry::TelemetryHub;
use capydeploy_protocol::telemetry::{
    BatteryMetrics, CpuMetrics, FanMetrics, GpuMetrics, MemoryMetrics, PowerMetrics, SteamStatus,
};

use crate::message::Message;
use crate::theme;

// Color thresholds for usage and temperature.
const GREEN: Color = Color::from_rgb(0.18, 0.80, 0.44);
const YELLOW: Color = Color::from_rgb(0.90, 0.75, 0.10);
const RED: Color = Color::from_rgb(0.90, 0.25, 0.20);

/// Renders the telemetry dashboard with grouped metric cards.
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

    // If the agent explicitly disabled telemetry or data went stale, don't
    // show frozen metrics — that's confusing UX.
    if agent.is_some_and(|a| !a.enabled() || a.is_stale()) {
        content = content.push(
            widget::text("Telemetry disabled on agent.").class(theme::MUTED_TEXT),
        );
        return content.into();
    }

    let agent = agent.unwrap();
    let latest = agent.latest().unwrap();

    let mut cards: Vec<Element<'a, Message>> = Vec::new();

    if let Some(cpu) = &latest.cpu {
        cards.push(cpu_card(cpu));
    }

    if let Some(gpu) = &latest.gpu {
        cards.push(gpu_card(gpu));
    }

    if let Some(mem) = &latest.memory {
        cards.push(memory_card(mem));
    }

    if let Some(card) = system_card(&latest.power, &latest.battery, &latest.fan) {
        cards.push(card);
    }

    if let Some(steam) = &latest.steam {
        cards.push(steam_card(steam));
    }

    if !cards.is_empty() {
        // Manual 2-column layout so cards take their natural height
        // instead of stretching to match the tallest in the row.
        let mut left = widget::column().spacing(12).width(Length::Fill);
        let mut right = widget::column().spacing(12).width(Length::Fill);
        for (i, card) in cards.into_iter().enumerate() {
            if i % 2 == 0 {
                left = left.push(card);
            } else {
                right = right.push(card);
            }
        }
        content = content.push(
            widget::row()
                .spacing(12)
                .push(left)
                .push(right)
                .align_y(cosmic::iced::Alignment::Start),
        );
    }

    widget::scrollable(content).into()
}

// ---------------------------------------------------------------------------
// Grouped cards
// ---------------------------------------------------------------------------

fn cpu_card(cpu: &CpuMetrics) -> Element<'static, Message> {
    let mut col = widget::column()
        .spacing(8)
        .push(card_header("CPU"))
        .push(metric_row_bar(
            "Usage",
            &format!("{:.1}%", cpu.usage_percent),
            cpu.usage_percent / 100.0,
            usage_color(cpu.usage_percent),
        ));

    if cpu.temp_celsius >= 0.0 {
        col = col.push(metric_row_text(
            "Temperature",
            &format!("{:.0}°C", cpu.temp_celsius),
            temp_color(cpu.temp_celsius),
        ));
    }

    if cpu.freq_m_hz >= 0.0 {
        col = col.push(metric_row_text(
            "Frequency",
            &format_freq(cpu.freq_m_hz),
            Color::WHITE,
        ));
    }

    card_container(col.into())
}

fn gpu_card(gpu: &GpuMetrics) -> Element<'static, Message> {
    let mut col = widget::column().spacing(8).push(card_header("GPU"));

    if gpu.usage_percent >= 0.0 {
        col = col.push(metric_row_bar(
            "Usage",
            &format!("{:.1}%", gpu.usage_percent),
            gpu.usage_percent / 100.0,
            usage_color(gpu.usage_percent),
        ));
    }

    if gpu.temp_celsius >= 0.0 {
        col = col.push(metric_row_text(
            "Temperature",
            &format!("{:.0}°C", gpu.temp_celsius),
            temp_color(gpu.temp_celsius),
        ));
    }

    if gpu.freq_m_hz >= 0.0 {
        col = col.push(metric_row_text(
            "Core Freq",
            &format_freq(gpu.freq_m_hz),
            Color::WHITE,
        ));
    }

    if gpu.mem_freq_m_hz > 0.0 {
        col = col.push(metric_row_text(
            "Mem Freq",
            &format_freq(gpu.mem_freq_m_hz),
            Color::WHITE,
        ));
    }

    if gpu.vram_total_bytes > 0 {
        let used_mb = gpu.vram_used_bytes as f64 / 1_048_576.0;
        let total_mb = gpu.vram_total_bytes as f64 / 1_048_576.0;
        let ratio = used_mb / total_mb;
        col = col.push(metric_row_bar(
            "VRAM",
            &format!("{:.0}/{:.0} MB", used_mb, total_mb),
            ratio,
            usage_color(ratio * 100.0),
        ));
    }

    card_container(col.into())
}

fn memory_card(mem: &MemoryMetrics) -> Element<'static, Message> {
    let total_gb = mem.total_bytes as f64 / 1_073_741_824.0;
    let avail_gb = mem.available_bytes as f64 / 1_073_741_824.0;
    let used_gb = total_gb - avail_gb;

    let mut col = widget::column()
        .spacing(8)
        .push(card_header("Memory"))
        .push(metric_row_bar(
            "Usage",
            &format!("{:.1}%", mem.usage_percent),
            mem.usage_percent / 100.0,
            usage_color(mem.usage_percent),
        ))
        .push(metric_row_text(
            "Used / Total",
            &format!("{:.1} / {:.1} GB", used_gb, total_gb),
            Color::WHITE,
        ));

    if mem.swap_total_bytes > 0 {
        let swap_total = mem.swap_total_bytes as f64 / 1_073_741_824.0;
        let swap_free = mem.swap_free_bytes as f64 / 1_073_741_824.0;
        let swap_used = swap_total - swap_free;
        let swap_ratio = if swap_total > 0.0 {
            swap_used / swap_total
        } else {
            0.0
        };
        col = col.push(divider());
        col = col.push(metric_row_bar(
            "Swap",
            &format!("{:.1} / {:.1} GB", swap_used, swap_total),
            swap_ratio,
            usage_color(swap_ratio * 100.0),
        ));
    }

    card_container(col.into())
}

fn system_card(
    power: &Option<PowerMetrics>,
    battery: &Option<BatteryMetrics>,
    fan: &Option<FanMetrics>,
) -> Option<Element<'static, Message>> {
    let has_power = power.as_ref().is_some_and(|p| p.power_watts > 0.0);
    let has_battery = battery.is_some();
    let has_fan = fan.is_some();

    if !has_power && !has_battery && !has_fan {
        return None;
    }

    let mut col = widget::column().spacing(8).push(card_header("System"));
    let mut has_prev_section = false;

    if let Some(power) = power
        && power.power_watts > 0.0
    {
        col = col.push(metric_row_text(
            "Power Draw",
            &format!("{:.1} W", power.power_watts),
            Color::WHITE,
        ));
        if power.tdp_watts > 0.0 {
            let ratio = power.power_watts / power.tdp_watts;
            col = col.push(
                widget::progress_bar(0.0..=1.0, (ratio as f32).clamp(0.0, 1.0))
                    .height(6)
                    .class(theme::colored_progress_bar(usage_color(ratio * 100.0))),
            );
            col = col.push(metric_row_text(
                "TDP Limit",
                &format!("{:.1} W", power.tdp_watts),
                Color::WHITE,
            ));
        }
        has_prev_section = true;
    }

    if let Some(bat) = battery {
        if has_prev_section {
            col = col.push(divider());
        }
        let cap = bat.capacity as f64;
        col = col.push(metric_row_text(
            "Battery",
            &format!("{}%", bat.capacity),
            Color::WHITE,
        ));
        col = col.push(
            widget::progress_bar(0.0..=1.0, (cap as f32 / 100.0).clamp(0.0, 1.0))
                .height(6)
                .class(theme::colored_progress_bar(usage_color(100.0 - cap))),
        );
        col = col.push(metric_row_text("Status", &bat.status, Color::WHITE));
        has_prev_section = true;
    }

    if let Some(fan) = fan {
        if has_prev_section {
            col = col.push(divider());
        }
        col = col.push(metric_row_text(
            "Fan",
            &format!("{} RPM", fan.rpm),
            Color::WHITE,
        ));
    }

    Some(card_container(col.into()))
}

fn steam_card(steam: &SteamStatus) -> Element<'static, Message> {
    let (status_text, status_color) = if steam.running {
        ("Running", GREEN)
    } else {
        ("Not Running", theme::MUTED_TEXT)
    };

    let mut col = widget::column()
        .spacing(8)
        .push(card_header("Steam"))
        .push(metric_row_text("Status", status_text, status_color));

    if steam.gaming_mode {
        col = col.push(metric_row_text("Mode", "Gaming Mode", theme::CYAN));
    }

    card_container(col.into())
}

// ---------------------------------------------------------------------------
// Reusable helpers
// ---------------------------------------------------------------------------

fn card_header(title: &str) -> Element<'static, Message> {
    widget::text::title4(title.to_string())
        .class(theme::CYAN)
        .into()
}

/// Row with label + value text + colored progress bar underneath.
fn metric_row_bar(
    label: &str,
    value_text: &str,
    ratio: f64,
    color: Color,
) -> Element<'static, Message> {
    let clamped = (ratio as f32).clamp(0.0, 1.0);

    widget::column()
        .push(
            widget::row()
                .push(widget::text::body(label.to_string()).class(theme::MUTED_TEXT))
                .push(horizontal_space())
                .push(widget::text::body(value_text.to_string())),
        )
        .push(
            widget::progress_bar(0.0..=1.0, clamped)
                .height(6)
                .class(theme::colored_progress_bar(color)),
        )
        .spacing(4)
        .into()
}

/// Row with label + colored value text (no progress bar).
fn metric_row_text(
    label: &str,
    value_text: &str,
    value_color: Color,
) -> Element<'static, Message> {
    widget::row()
        .push(widget::text::body(label.to_string()).class(theme::MUTED_TEXT))
        .push(horizontal_space())
        .push(widget::text::body(value_text.to_string()).class(value_color))
        .into()
}

/// Horizontal divider line between card sections.
fn divider() -> Element<'static, Message> {
    use cosmic::iced::Background;
    container(widget::vertical_space().height(1))
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(
            |_theme: &cosmic::Theme| cosmic::iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(0.278, 0.333, 0.412, 0.5))),
                ..Default::default()
            },
        )))
        .into()
}

/// Wraps content in a card container with `canvas_bg` styling.
fn card_container(content: Element<'static, Message>) -> Element<'static, Message> {
    container(content)
        .padding(16)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(
            theme::canvas_bg,
        )))
        .into()
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

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
    } else if percent < 85.0 {
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

