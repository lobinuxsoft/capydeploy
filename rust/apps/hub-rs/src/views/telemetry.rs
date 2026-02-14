//! Telemetry dashboard — real-time gauges, sparklines, and system info cards.

use cosmic::iced::Length;
use cosmic::widget::{self, canvas, container};
use cosmic::Element;

use capydeploy_hub_telemetry::TelemetryHub;

use crate::message::Message;
use crate::theme;

use super::TelemetryWidgets;

/// Renders the telemetry dashboard.
pub fn view<'a>(
    hub: &'a TelemetryHub,
    agent_id: Option<&str>,
    widgets: &'a TelemetryWidgets,
) -> Element<'a, Message> {
    let mut content = widget::column().spacing(16);

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

    // -- Gauges row --
    let gauges = widget::row()
        .push(gauge_card(&widgets.cpu_gauge))
        .push(gauge_card(&widgets.gpu_gauge))
        .push(gauge_card(&widgets.cpu_temp_gauge))
        .push(gauge_card(&widgets.mem_gauge))
        .spacing(12);
    content = content.push(gauges);

    // -- Sparklines --
    content = content.push(sparkline_card("CPU Usage", &widgets.cpu_sparkline));
    content = content.push(sparkline_card("GPU Usage", &widgets.gpu_sparkline));
    content = content.push(sparkline_card("Memory Usage", &widgets.mem_sparkline));

    // -- System info cards --
    if let Some(latest) = agent.latest() {
        let mut info_items: Vec<Element<'_, Message>> = Vec::new();

        if let Some(cpu) = &latest.cpu {
            info_items.push(info_chip(format!("CPU Freq: {:.0} MHz", cpu.freq_m_hz)));
        }

        if let Some(gpu) = &latest.gpu {
            let mut gpu_text = format!("GPU Freq: {:.0} MHz", gpu.freq_m_hz);
            if gpu.vram_total_bytes > 0 {
                let used_mb = gpu.vram_used_bytes as f64 / 1_048_576.0;
                let total_mb = gpu.vram_total_bytes as f64 / 1_048_576.0;
                gpu_text.push_str(&format!(" | VRAM: {:.0}/{:.0} MB", used_mb, total_mb));
            }
            info_items.push(info_chip(gpu_text));
        }

        if let Some(mem) = &latest.memory {
            let total_gb = mem.total_bytes as f64 / 1_073_741_824.0;
            let avail_gb = mem.available_bytes as f64 / 1_073_741_824.0;
            let used_gb = total_gb - avail_gb;
            info_items.push(info_chip(format!(
                "RAM: {:.1}/{:.1} GB ({:.0}%)",
                used_gb, total_gb, mem.usage_percent
            )));
        }

        if let Some(bat) = &latest.battery {
            info_items.push(info_chip(format!(
                "Battery: {}% ({})",
                bat.capacity, bat.status
            )));
        }

        if let Some(power) = &latest.power {
            info_items.push(info_chip(format!(
                "Power: {:.1}W / {:.1}W TDP",
                power.power_watts, power.tdp_watts
            )));
        }

        if let Some(fan) = &latest.fan {
            info_items.push(info_chip(format!("Fan: {} RPM", fan.rpm)));
        }

        if let Some(steam) = &latest.steam {
            let status = if steam.gaming_mode {
                "Gaming Mode"
            } else if steam.running {
                "Running"
            } else {
                "Not running"
            };
            info_items.push(info_chip(format!("Steam: {status}")));
        }

        if !info_items.is_empty() {
            let mut info_row = widget::row().spacing(8);
            for item in info_items {
                info_row = info_row.push(item);
            }

            content = content.push(
                container(info_row.padding(12))
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Custom(Box::new(
                        theme::canvas_bg,
                    ))),
            );
        }
    }

    widget::scrollable(content).into()
}

/// Wraps a gauge widget in a styled card container.
fn gauge_card<'a>(gauge: &'a capydeploy_hub_widgets::TelemetryGauge) -> Element<'a, Message> {
    let canvas_widget = canvas::Canvas::new(gauge)
        .width(Length::Fixed(150.0))
        .height(Length::Fixed(150.0));

    container(canvas_widget)
        .padding(8)
        .class(cosmic::theme::Container::Custom(Box::new(
            theme::canvas_bg,
        )))
        .into()
}

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

/// Small text chip for system info values.
fn info_chip(text: String) -> Element<'static, Message> {
    widget::text::caption(text).class(theme::MUTED_TEXT).into()
}
