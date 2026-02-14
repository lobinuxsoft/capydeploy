//! Hub application — `cosmic::Application` implementation.

use cosmic::app::Core;
use cosmic::iced::widget::container as iced_container;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, container};
use cosmic::{Application, Element};

use crate::config::HubConfig;
use crate::message::{Message, NavPage};
use crate::theme;

/// Main Hub application state.
pub struct Hub {
    core: Core,
    config: HubConfig,
    nav_page: NavPage,
    is_connected: bool,
}

impl Application for Hub {
    type Executor = cosmic::executor::Default;
    type Message = Message;
    type Flags = HubConfig;

    const APP_ID: &'static str = "com.capydeploy.hub";

    fn init(mut core: Core, config: HubConfig) -> (Self, cosmic::app::Task<Message>) {
        // Disable COSMIC CSD header — KDE/other WMs provide their own buttons.
        core.window.show_headerbar = false;

        let app = Self {
            core,
            config,
            nav_page: NavPage::Devices,
            is_connected: false,
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
            Message::NavigateTo(page) => {
                if !page.requires_connection() || self.is_connected {
                    self.nav_page = page;
                }
            }
            Message::Tick => {}
        }
        cosmic::app::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();
        let content = self.view_content();

        widget::row()
            .push(sidebar)
            .push(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Hub {
    /// Renders the sidebar with navigation buttons.
    fn view_sidebar(&self) -> Element<'_, Message> {
        let mut nav = widget::column().spacing(4).padding([16, 8]);

        // Brand header.
        let header = widget::column()
            .push(widget::text::title4("CapyDeploy").class(theme::CYAN))
            .push(widget::text::caption(&self.config.name).class(theme::MUTED_TEXT))
            .spacing(2)
            .padding([0, 8, 16, 8]);
        nav = nav.push(header);

        // Nav items.
        for page in NavPage::ALL {
            let is_active = self.nav_page == page;
            let disabled = page.requires_connection() && !self.is_connected;

            let btn = if disabled {
                let label = widget::text(page.label()).class(theme::MUTED_TEXT);
                widget::button::custom(label)
            } else if is_active {
                let label = widget::text(page.label()).class(theme::CYAN);
                widget::button::custom(label).on_press(Message::NavigateTo(page))
            } else {
                let label = widget::text(page.label());
                widget::button::custom(label).on_press(Message::NavigateTo(page))
            };

            let btn_element: Element<'_, Message> = btn.width(Length::Fill).into();

            if is_active {
                nav = nav.push(
                    container(btn_element)
                        .class(cosmic::theme::Container::Custom(Box::new(
                            theme::nav_active_bg,
                        ))),
                );
            } else {
                nav = nav.push(btn_element);
            }
        }

        // Connection status at bottom.
        nav = nav.push(widget::Space::with_height(Length::Fill));

        let status_text = if self.is_connected {
            "Connected"
        } else {
            "No agent connected"
        };
        let status_color = if self.is_connected {
            capydeploy_hub_widgets::color_for_ratio(
                0.0,
                &capydeploy_hub_widgets::GaugeThresholds::default(),
            )
        } else {
            theme::MUTED_TEXT
        };
        nav = nav.push(
            widget::text::caption(status_text)
                .class(status_color)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        );

        container(nav)
            .width(Length::Fixed(200.0))
            .height(Length::Fill)
            .class(cosmic::theme::Container::Custom(Box::new(
                theme::sidebar_bg,
            )))
            .into()
    }

    /// Renders the main content area based on the active nav page.
    fn view_content(&self) -> Element<'_, Message> {
        let content = match self.nav_page {
            NavPage::Devices => self.view_devices(),
            NavPage::Deploy => self.view_placeholder("Deploy"),
            NavPage::Games => self.view_placeholder("Games"),
            NavPage::Telemetry => self.view_placeholder("Telemetry"),
            NavPage::Console => self.view_placeholder("Console"),
            NavPage::Settings => self.view_placeholder("Settings"),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
            .class(cosmic::theme::Container::Custom(Box::new(content_bg)))
            .into()
    }

    /// Placeholder view for pages not yet implemented.
    fn view_placeholder(&self, name: &'static str) -> Element<'_, Message> {
        widget::column()
            .push(widget::text::title3(name))
            .push(widget::text("Coming soon...").class(theme::MUTED_TEXT))
            .spacing(8)
            .into()
    }

    /// Devices page — skeleton for now.
    fn view_devices(&self) -> Element<'_, Message> {
        widget::column()
            .push(widget::text::title3("Devices"))
            .push(
                widget::text("Searching for agents on the network...").class(theme::MUTED_TEXT),
            )
            .spacing(8)
            .into()
    }
}

/// Content area background.
fn content_bg(_theme: &cosmic::Theme) -> iced_container::Style {
    iced_container::Style {
        background: Some(cosmic::iced::Background::Color(theme::DARK_BG)),
        ..Default::default()
    }
}
