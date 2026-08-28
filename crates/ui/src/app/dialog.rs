// SPDX-License-Identifier: GPL-3.0-only

use crate::app::{AppModel, Message};
use crate::fl;
use accounts_core::Uuid;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::theme::spacing;
use cosmic::widget;

#[derive(Clone, Debug)]
pub enum DialogPage {
    /// Lets the user pick which provider to sign in with.
    AddAccount,
    /// Waits for a sign-in that is happening in the browser, for the given provider.
    SigningIn(String),
    /// Confirms removal of the account with the given id and label.
    RemoveAccount(Uuid, String),
}

impl DialogPage {
    pub(super) fn view<'a>(&'a self, app: &'a AppModel) -> Element<'a, Message> {
        match self {
            DialogPage::AddAccount => widget::dialog()
                .title(fl!("add-account-title"))
                .body(fl!("add-account-body"))
                .control(app.provider_list())
                .primary_action(
                    widget::button::standard(fl!("cancel")).on_press(Message::CloseDialog),
                )
                .into(),

            DialogPage::SigningIn(provider) => {
                let provider_name = app.provider_name(provider);

                widget::dialog()
                .title(fl!("signing-in-title"))
                .body(fl!("signing-in-body", provider = provider_name.as_str()))
                .icon(app.provider_icon(provider, 48))
                .control(
                    widget::progress_bar::indeterminate_linear()
                        .width(Length::Fill)
                        .apply(widget::container)
                        .padding([spacing().space_s, 0]),
                )
                .primary_action(
                    widget::button::standard(fl!("cancel")).on_press(Message::CancelAuth),
                )
                .secondary_action(
                    widget::button::text(fl!("open-browser-again")).on_press(Message::OpenAuthUrl),
                )
                .into()
            }

            DialogPage::RemoveAccount(id, label) => widget::dialog()
                .title(fl!("remove-account-title", account = label.as_str()))
                .body(fl!("remove-account-body"))
                .icon(
                    widget::icon::from_name("dialog-warning-symbolic")
                        .size(32)
                        .apply(widget::container)
                        .align_y(Alignment::Center),
                )
                .primary_action(
                    widget::button::destructive(fl!("remove"))
                        .on_press(Message::RemoveAccount(*id)),
                )
                .secondary_action(
                    widget::button::standard(fl!("cancel")).on_press(Message::CloseDialog),
                )
                .into(),
        }
    }
}
