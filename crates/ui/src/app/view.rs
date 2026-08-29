// SPDX-License-Identifier: GPL-3.0-only

//! The screens the window can show, and the pieces they share.

use crate::app::{AppModel, Message};
use crate::fl;
use accounts_core::Local;
use accounts_core::models::{Account, IconSource, Service};
use chrono::{DateTime, Utc};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::theme::spacing;
use cosmic::widget;

const APP_ICON: &[u8] = include_bytes!("../../resources/icons/hicolor/scalable/apps/icon.svg");
const GOOGLE_ICON: &[u8] = include_bytes!("../../resources/img/google.png");
const MICROSOFT_ICON: &[u8] = include_bytes!("../../resources/img/microsoft.png");

/// How wide a settings page is allowed to grow before it stops stretching.
const CONTENT_WIDTH: f32 = 720.0;
/// How wide a block of explanatory text is allowed to grow.
const PROSE_WIDTH: f32 = 520.0;
/// How wide the provider picker is allowed to grow on the welcome screen.
const PICKER_WIDTH: f32 = 360.0;

impl AppModel {
    /// The first thing someone sees when they have no accounts yet.
    pub(super) fn welcome_view(&self) -> Element<'_, Message> {
        if self.providers.is_empty() {
            return self.empty_state(
                "dialog-warning-symbolic",
                fl!("no-account-providers"),
                fl!("no-account-providers-body"),
                Some(
                    widget::button::suggested(fl!("try-again"))
                        .on_press(Message::CreateClient)
                        .into(),
                ),
            );
        }

        let header = widget::Column::new()
            .push(
                widget::icon(widget::icon::from_svg_bytes(APP_ICON))
                    .size(96)
                    .width(Length::Fixed(96.0))
                    .height(Length::Fixed(96.0)),
            )
            .push(widget::text::title1(fl!("app-title")).align_x(Horizontal::Center))
            .push(
                widget::text::body(fl!("connect-accounts"))
                    .align_x(Horizontal::Center)
                    .width(Length::Fixed(PROSE_WIDTH)),
            )
            .align_x(Alignment::Center)
            .spacing(spacing().space_s);

        widget::Column::new()
            .push(header)
            .push(
                self.provider_list()
                    .apply(widget::container)
                    .max_width(PICKER_WIDTH),
            )
            .align_x(Alignment::Center)
            .spacing(spacing().space_l)
            .apply(widget::container)
            .center(Length::Fill)
            .padding(spacing().space_l)
            .into()
    }

    /// Shown when accounts exist but the nav bar selection was cleared.
    pub(super) fn no_selection_view(&self) -> Element<'_, Message> {
        self.empty_state(
            "system-users-symbolic",
            fl!("no-account-selected"),
            fl!("no-account-selected-body"),
            None,
        )
    }

    /// Everything there is to know about, and change on, a single account.
    pub(super) fn account_view<'a>(&'a self, account: &'a Account) -> Element<'a, Message> {
        let mut sections = vec![
            self.account_header(account),
            self.account_section(account),
            self.services_section(account),
            self.details_section(account),
        ];

        if !account.enabled {
            sections.insert(1, warning_banner(fl!("account-disabled-warning")));
        }

        widget::settings::view_column(sections)
            .apply(widget::container)
            .max_width(CONTENT_WIDTH)
            .padding(spacing().space_m)
            .apply(widget::container)
            .center_x(Length::Fill)
            .apply(widget::scrollable)
            .height(Length::Fill)
            .into()
    }

    fn account_header<'a>(&'a self, account: &'a Account) -> Element<'a, Message> {
        let label = self.account_label(account);
        let title = if account.display_name.is_empty() {
            label.clone()
        } else {
            account.display_name.clone()
        };

        let mut names = widget::Column::new()
            .push(widget::text::title3(title.clone()))
            .spacing(spacing().space_xxxs);

        // The subtitle only earns its space when it says something the title did not.
        if label != title {
            names = names.push(widget::text::body(label).class(cosmic::style::Text::Default));
        }

        widget::Row::new()
            .push(self.provider_icon(&account.provider, 56))
            .push(names)
            .spacing(spacing().space_s)
            .align_y(Alignment::Center)
            .into()
    }

    fn account_section<'a>(&self, account: &'a Account) -> Element<'a, Message> {
        widget::settings::section()
            .title(fl!("account"))
            .add(
                widget::settings::item::builder(fl!("account-enabled"))
                    .description(fl!("account-enabled-description"))
                    .toggler(account.enabled, Message::SetAccountEnabled),
            )
            .into()
    }

    fn services_section<'a>(&self, account: &'a Account) -> Element<'a, Message> {
        let mut section = widget::settings::section().header(
            widget::Column::new()
                .push(widget::text::heading(fl!("services")))
                .push(
                    widget::text::caption(fl!("services-description"))
                        .class(cosmic::style::Text::Default),
                )
                .spacing(spacing().space_xxxs),
        );

        for (service, enabled) in &account.services {
            let (name, description, icon) = service_details(service);
            let service = service.clone();

            section = section.add(
                widget::settings::item::builder(name)
                    .description(description)
                    .icon(widget::icon::from_name(icon).size(16))
                    .toggler(*enabled, move |enabled| {
                        Message::SetServiceEnabled(service.clone(), enabled)
                    }),
            );
        }

        section.into()
    }

    fn details_section<'a>(&self, account: &'a Account) -> Element<'a, Message> {
        widget::settings::section()
            .title(fl!("details"))
            .add(widget::settings::flex_item(
                fl!("provider"),
                widget::text::body(self.provider_name(&account.provider)),
            ))
            .add(widget::settings::flex_item(
                fl!("created-at"),
                widget::text::body(format_timestamp(account.created_at)),
            ))
            .add(widget::settings::flex_item(
                fl!("last-used"),
                widget::text::body(
                    account
                        .last_used
                        .map(format_timestamp)
                        .unwrap_or_else(|| fl!("no-usage")),
                ),
            ))
            .into()
    }

    /// The provider picker, shared by the welcome screen and the add-account dialog.
    /// The provider picker, shared by the welcome screen and the add-account dialog.
    pub(super) fn provider_list(&self) -> Element<'_, Message> {
        if self.providers.is_empty() {
            return widget::text::body(fl!("no-account-providers"))
                .align_x(Horizontal::Center)
                .width(Length::Fill)
                .into();
        }

        let rows: Vec<Element<'_, Message>> = self
            .providers
            .iter()
            .map(|provider| {
                widget::Row::new()
                    .push(self.provider_icon(&provider.id, 32))
                    .push(widget::text::body(provider.name.clone()).width(Length::Fill))
                    .push(widget::icon::from_name("go-next-symbolic").size(16))
                    .spacing(spacing().space_s)
                    .align_y(Alignment::Center)
                    .apply(widget::button::custom)
                    .padding([spacing().space_s, spacing().space_m])
                    .width(Length::Fill)
                    .name(provider.name.clone())
                    .on_press(Message::StartAuth(provider.id.clone()))
                    .into()
            })
            .collect();

        widget::Column::with_children(rows)
            .spacing(spacing().space_xs)
            .width(Length::Fill)
            .into()
    }

    /// The provider's human-readable name, falling back to its identifier.
    pub(super) fn provider_name(&self, provider_id: &str) -> String {
        self.providers
            .iter()
            .find(|provider| provider.id == provider_id)
            .map_or_else(|| provider_id.to_string(), |provider| provider.name.clone())
    }

    pub(super) fn provider_icon(&self, provider_id: &str, size: u16) -> Element<'static, Message> {
        widget::icon(self.provider_icon_handle(provider_id))
            .size(size)
            .width(Length::Fixed(f32::from(size)))
            .height(Length::Fixed(f32::from(size)))
            .into()
    }

    /// Resolves a provider's icon, preferring what the provider advertises and
    /// falling back to the icons shipped with the application.
    pub(super) fn provider_icon_handle(&self, provider_id: &str) -> widget::icon::Handle {
        let source = self
            .providers
            .iter()
            .find(|provider| provider.id == provider_id)
            .and_then(|provider| provider.icon_source());

        match source {
            // A remote icon is only usable once it has finished downloading.
            Some(IconSource::Url(url)) => match self.icon_cache.get(&url) {
                Some(handle) => handle.clone(),
                None => bundled_provider_icon(provider_id),
            },
            Some(IconSource::Path(path)) => widget::icon::from_path(path),
            Some(IconSource::ThemeName(name)) => widget::icon::from_name(name.as_str()).into(),
            None => bundled_provider_icon(provider_id),
        }
    }

    fn empty_state<'a>(
        &self,
        icon: &'static str,
        title: String,
        body: String,
        action: Option<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        widget::Column::new()
            .push(widget::icon::from_name(icon).size(56))
            .push(widget::text::title3(title).align_x(Horizontal::Center))
            .push(
                widget::text::body(body)
                    .align_x(Horizontal::Center)
                    .width(Length::Fixed(PROSE_WIDTH))
                    .class(cosmic::style::Text::Default),
            )
            .push_maybe(action)
            .align_x(Alignment::Center)
            .spacing(spacing().space_s)
            .apply(widget::container)
            .center(Length::Fill)
            .padding(spacing().space_l)
            .into()
    }
}

fn warning_banner(message: String) -> Element<'static, Message> {
    widget::Row::new()
        .push(widget::icon::from_name("dialog-warning-symbolic").size(16))
        .push(widget::text::body(message).width(Length::Fill))
        .spacing(spacing().space_xs)
        .align_y(Alignment::Center)
        .apply(widget::container)
        .class(cosmic::style::Container::Card)
        .padding(spacing().space_s)
        .width(Length::Fill)
        .into()
}

fn bundled_provider_icon(provider_id: &str) -> widget::icon::Handle {
    match provider_id {
        "google" => widget::icon::from_raster_bytes(GOOGLE_ICON),
        "microsoft" => widget::icon::from_raster_bytes(MICROSOFT_ICON),
        _ => widget::icon::from_name("network-server-symbolic").into(),
    }
}

/// The name, explanation, and icon shown for a service in the account page.
fn service_details(service: &Service) -> (String, String, &'static str) {
    match service {
        Service::Email => (
            fl!("service-email"),
            fl!("service-email-description"),
            "mail-message-new-symbolic",
        ),
        Service::Calendar => (
            fl!("service-calendar"),
            fl!("service-calendar-description"),
            "x-office-calendar-symbolic",
        ),
        Service::Contacts => (
            fl!("service-contacts"),
            fl!("service-contacts-description"),
            "x-office-address-book-symbolic",
        ),
        Service::Tasks => (
            fl!("service-tasks"),
            fl!("service-tasks-description"),
            "checkbox-checked-symbolic",
        ),
    }
}

fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp
        .with_timezone(&Local)
        .format("%B %d, %Y at %I:%M %p")
        .to_string()
}
