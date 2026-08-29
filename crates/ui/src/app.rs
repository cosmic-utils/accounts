// SPDX-License-Identifier: GPL-3.0-only

mod dialog;
mod view;

use crate::fl;
use accounts_core::models::{Account, DbusProviderInfo, Service};
use accounts_core::{AccountsClient, Uuid, zbus};
use cosmic::app::context_drawer;
use cosmic::iced::futures::channel::mpsc::Sender;
use cosmic::iced::keyboard::{Event as KeyEvent, Key, Modifiers};
use cosmic::iced::{Event, Subscription, event, stream};
use cosmic::prelude::*;
use cosmic::theme::spacing;
use cosmic::widget::menu::Action as _;
use cosmic::widget::{self, ToastId, menu, nav_bar};
use futures_util::{SinkExt, Stream, StreamExt};
use std::collections::{HashMap, VecDeque};

pub use dialog::DialogPage;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

pub struct AppModel {
    core: cosmic::Core,
    context_page: ContextPage,
    nav: nav_bar::Model,
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    dialog_pages: VecDeque<DialogPage>,
    toasts: widget::Toasts<Message>,
    about: widget::about::About,
    client: Option<AccountsClient>,
    accounts: Vec<Account>,
    providers: Vec<DbusProviderInfo>,
    icon_cache: HashMap<String, widget::icon::Handle>,
    selected_account: Option<Account>,
    /// The sign-in attempt currently waiting on the user's browser, if any.
    pending_auth: Option<PendingAuth>,
}

/// A sign-in that has been handed off to the browser and has not come back yet.
#[derive(Default)]
struct PendingAuth {
    /// The `Request` object tracking this attempt, once `CreateAccount` returns it.
    request_path: Option<zbus::zvariant::OwnedObjectPath>,
    /// The authorization URL, once the daemon has produced it. Kept around so the
    /// user can reopen the page if their browser never appeared.
    url: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Window chrome
    ToggleContextPage(ContextPage),
    OpenDialog(DialogPage),
    CloseDialog,
    LaunchUrl(String),
    ShowToast(String),
    CloseToast(ToastId),
    Key(Modifiers, Key),

    // Accounts and providers
    CreateClient,
    SetClient(Option<AccountsClient>),
    LoadAccounts,
    SetAccounts(Vec<Account>),
    SetProviders(Vec<DbusProviderInfo>),
    ProviderIconLoaded(String, Vec<u8>),
    SelectAccount(Account),
    SetAccountEnabled(bool),
    SetServiceEnabled(Service, bool),
    RemoveAccount(Uuid),
    AccountRemoved(Uuid, String),

    // Sign-in
    StartAuth(String),
    RequestCreated(zbus::zvariant::OwnedObjectPath),
    AuthUrlReady(String),
    OpenAuthUrl,
    CancelAuth,
    AuthFailed(String),
    AccountAdded(Uuid),
    AccountReady(Account),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "dev.edfloreshz.Accounts";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav: nav_bar::Model::default(),
            key_binds: key_binds(),
            toasts: widget::toaster::Toasts::new(Message::CloseToast),
            dialog_pages: VecDeque::new(),
            about: about(),
            client: None,
            accounts: Vec::new(),
            providers: Vec::new(),
            icon_cache: HashMap::new(),
            selected_account: None,
            pending_auth: None,
        };

        let tasks = vec![
            app.update_title(),
            cosmic::task::message(Message::CreateClient),
        ];

        (app, Task::batch(tasks))
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![
            menu::Tree::with_children(
                menu::root(fl!("file")).apply(Element::from),
                menu::items(
                    &self.key_binds,
                    vec![menu::Item::Button(
                        fl!("add-account"),
                        None,
                        MenuAction::AddAccount,
                    )],
                ),
            ),
            menu::Tree::with_children(
                menu::root(fl!("view")).apply(Element::from),
                menu::items(
                    &self.key_binds,
                    vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
                ),
            ),
        ])
        .item_width(menu::ItemWidth::Uniform(240))
        .item_height(menu::ItemHeight::Uniform(40));

        vec![menu_bar.into()]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            widget::tooltip(
                widget::button::icon(widget::icon::from_name("list-add-symbolic"))
                    .on_press(Message::OpenDialog(DialogPage::AddAccount))
                    .name(fl!("add-account")),
                widget::text(fl!("add-account")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        ]
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        (!self.accounts.is_empty()).then_some(&self.nav)
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        self.dialog_pages.front().map(|page| page.view(self))
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        self.nav.activate(id);

        let mut tasks = vec![self.update_title()];
        if let Some(account) = self.nav.active_data::<Account>().cloned() {
            tasks.push(self.update(Message::SelectAccount(account)));
        }
        Task::batch(tasks)
    }

    fn on_escape(&mut self) -> cosmic::app::Task<Self::Message> {
        // A sign-in dialog is only meaningful while we are still waiting on the
        // browser, so dismissing it also gives up on the attempt.
        if matches!(self.dialog_pages.front(), Some(DialogPage::SigningIn(_))) {
            self.pending_auth = None;
        }

        if self.dialog_pages.pop_front().is_some() {
            return Task::none();
        }

        self.core.window.show_context = false;

        Task::none()
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(fl!("about")),
        })
    }

    fn footer(&self) -> Option<Element<'_, Self::Message>> {
        let account = self.selected_account.as_ref()?;

        Some(
            widget::Row::new()
                .push(widget::space::horizontal())
                .push(
                    widget::button::standard(fl!("remove-account"))
                        .leading_icon(widget::icon::from_name("user-trash-symbolic"))
                        .on_press(Message::OpenDialog(DialogPage::RemoveAccount(
                            account.id,
                            self.account_label(account),
                        ))),
                )
                .align_y(cosmic::iced::Alignment::Center)
                .apply(widget::container)
                .class(cosmic::style::Container::Card)
                .padding(spacing().space_xs)
                .into(),
        )
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let content = match self.selected_account.as_ref() {
            Some(account) => self.account_view(account),
            None if self.accounts.is_empty() => self.welcome_view(),
            None => self.no_selection_view(),
        };

        widget::toaster(&self.toasts, content)
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subscriptions = vec![event::listen_with(|event, status, _| match event {
            Event::Keyboard(KeyEvent::KeyPressed { key, modifiers, .. })
                if status == event::Status::Ignored =>
            {
                Some(Message::Key(modifiers, key))
            }
            _ => None,
        })];

        if self.client.is_some() {
            subscriptions.extend(DAEMON_SIGNALS.map(|signal| {
                Subscription::run_with(signal, |signal: &&'static str| {
                    let signal = *signal;
                    stream::channel(4, move |output| watch_daemon_signal(signal, output))
                })
            }));
        }

        if let Some(path) = self
            .pending_auth
            .as_ref()
            .and_then(|pending| pending.request_path.clone())
        {
            subscriptions.push(Subscription::run_with(
                path.clone(),
                |path: &zbus::zvariant::OwnedObjectPath| {
                    let path = path.clone();
                    stream::channel(4, move |output| watch_request(path, output))
                },
            ));
        }

        Subscription::batch(subscriptions)
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        let mut tasks = vec![];

        match message {
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }
            Message::OpenDialog(page) => {
                self.dialog_pages.clear();
                self.dialog_pages.push_back(page);
            }
            Message::CloseDialog => {
                self.dialog_pages.pop_front();
            }
            Message::LaunchUrl(url) => {
                if let Err(err) = open::that_detached(&url) {
                    tracing::error!("Failed to open {url:?}: {err}");
                }
            }
            Message::ShowToast(message) => {
                tasks.push(
                    self.toasts
                        .push(widget::toaster::Toast::new(message))
                        .map(cosmic::Action::App),
                );
            }
            Message::CloseToast(id) => self.toasts.remove(id),
            Message::Key(modifiers, key) => {
                let action = self.key_binds.iter().find_map(|(key_bind, action)| {
                    key_bind.matches(modifiers, &key, None).then_some(*action)
                });
                if let Some(action) = action {
                    return self.update(action.message());
                }
            }

            Message::CreateClient => {
                tasks.push(Task::perform(
                    async {
                        AccountsClient::new()
                            .await
                            .inspect_err(|err| tracing::error!("Failed to reach the daemon: {err}"))
                            .ok()
                    },
                    |client| cosmic::action::app(Message::SetClient(client)),
                ));
            }
            Message::SetClient(client) => {
                self.client = client;

                if let Some(client) = self.client.clone() {
                    tasks.push(cosmic::task::message(Message::LoadAccounts));
                    tasks.push(Task::perform(
                        async move {
                            client.list_providers().await.unwrap_or_else(|err| {
                                tracing::error!("Failed to list providers: {err}");
                                Vec::new()
                            })
                        },
                        |providers| cosmic::action::app(Message::SetProviders(providers)),
                    ));
                }
            }
            Message::LoadAccounts => {
                if let Some(client) = self.client.clone() {
                    tasks.push(Task::perform(
                        async move { client.list_accounts().await },
                        |accounts| match accounts {
                            Ok(accounts) => cosmic::action::app(Message::SetAccounts(accounts)),
                            Err(err) => {
                                tracing::error!("Failed to list accounts: {err}");
                                cosmic::action::none()
                            }
                        },
                    ));
                }
            }
            Message::SetAccounts(accounts) => {
                let was_empty = self.accounts.is_empty();
                let selected_id = self.selected_account.as_ref().map(|account| account.id);

                self.accounts = accounts;

                // Keep pointing at the same account so a background refresh does not
                // move the user, and fall back to the first one so a populated app
                // never opens on the "no accounts yet" screen.
                self.selected_account = selected_id
                    .and_then(|id| self.accounts.iter().find(|account| account.id == id))
                    .or_else(|| self.accounts.first())
                    .cloned();

                self.rebuild_nav();

                // Open the sidebar the moment there is something to put in it, but
                // otherwise leave whatever the user chose alone.
                if was_empty && !self.accounts.is_empty() {
                    self.core.nav_bar_set_toggled(true);
                }

                tasks.push(self.update_title());
            }
            Message::SetProviders(mut providers) => {
                providers.sort_by(|a, b| a.name.cmp(&b.name));

                for provider in &providers {
                    let Some(icon) = &provider.icon else { continue };
                    if !icon.starts_with("http://") && !icon.starts_with("https://") {
                        continue;
                    }
                    if self.icon_cache.contains_key(icon) {
                        continue;
                    }

                    let url = icon.clone();
                    tasks.push(Task::perform(
                        async move {
                            let bytes = reqwest::get(&url).await.ok()?.bytes().await.ok()?.to_vec();
                            Some((url, bytes))
                        },
                        |result| match result {
                            Some((url, bytes)) => {
                                cosmic::action::app(Message::ProviderIconLoaded(url, bytes))
                            }
                            None => cosmic::action::none(),
                        },
                    ));
                }

                self.providers = providers;
                self.rebuild_nav();
            }
            Message::ProviderIconLoaded(url, bytes) => {
                self.icon_cache
                    .insert(url, widget::icon::from_raster_bytes(bytes));
                self.rebuild_nav();
            }
            Message::SelectAccount(account) => self.selected_account = Some(account),
            Message::SetAccountEnabled(enabled) => {
                if let (Some(mut client), Some(account)) =
                    (self.client.clone(), self.selected_account.clone())
                {
                    tasks.push(Task::perform(
                        async move { client.set_account_enabled(&account.id, enabled).await },
                        |result: Result<(), zbus::fdo::Error>| match result {
                            Ok(()) => cosmic::action::app(Message::LoadAccounts),
                            Err(err) => {
                                tracing::error!("Failed to toggle account: {err}");
                                cosmic::action::none()
                            }
                        },
                    ));
                }
            }
            Message::SetServiceEnabled(service, enabled) => {
                if let (Some(mut client), Some(account)) =
                    (self.client.clone(), self.selected_account.clone())
                {
                    tasks.push(Task::perform(
                        async move {
                            client
                                .set_service_enabled(&account.id, &service, enabled)
                                .await
                        },
                        |result: Result<(), zbus::fdo::Error>| match result {
                            Ok(()) => cosmic::action::app(Message::LoadAccounts),
                            Err(err) => {
                                tracing::error!("Failed to set service: {err}");
                                cosmic::action::none()
                            }
                        },
                    ));
                }
            }
            Message::RemoveAccount(account_id) => {
                self.dialog_pages.pop_front();

                let label = self
                    .accounts
                    .iter()
                    .find(|account| account.id == account_id)
                    .map(|account| self.account_label(account))
                    .unwrap_or_default();

                if let Some(mut client) = self.client.clone() {
                    tasks.push(Task::perform(
                        async move { client.remove_account(&account_id).await },
                        move |result: Result<(), zbus::fdo::Error>| match result {
                            Ok(()) => cosmic::action::app(Message::AccountRemoved(
                                account_id,
                                label.clone(),
                            )),
                            Err(err) => {
                                tracing::error!("Failed to remove account: {err}");
                                cosmic::action::none()
                            }
                        },
                    ));
                }
            }
            Message::AccountRemoved(account_id, label) => {
                self.accounts.retain(|account| account.id != account_id);
                if self
                    .selected_account
                    .as_ref()
                    .is_some_and(|a| a.id == account_id)
                {
                    self.selected_account = None;
                }
                tasks.push(self.update(Message::ShowToast(fl!(
                    "account-removed",
                    account = label.as_str()
                ))));
                tasks.push(self.update(Message::LoadAccounts));
            }

            Message::StartAuth(provider) => {
                tracing::info!("Starting authentication for provider: {provider}");

                let Some(mut client) = self.client.clone() else {
                    return self.update(Message::AuthFailed(fl!("service-unavailable")));
                };

                self.pending_auth = Some(PendingAuth::default());
                tasks.push(
                    self.update(Message::OpenDialog(DialogPage::SigningIn(provider.clone()))),
                );
                tasks.push(Task::perform(
                    async move { client.create_account(&provider).await },
                    |result| match result {
                        Ok(proxy) => cosmic::action::app(Message::RequestCreated(
                            proxy.inner().path().to_owned().into(),
                        )),
                        Err(err) => {
                            tracing::error!("Failed to start authentication: {err}");
                            cosmic::action::app(Message::AuthFailed(err.to_string()))
                        }
                    },
                ));
            }
            Message::RequestCreated(path) => {
                let Some(pending) = self.pending_auth.as_mut() else {
                    // The user gave up while the daemon was preparing the request.
                    return Task::none();
                };
                pending.request_path = Some(path);
            }
            Message::AuthUrlReady(url) => {
                let Some(pending) = self.pending_auth.as_mut() else {
                    // The user gave up while the daemon was preparing the URL.
                    return Task::none();
                };
                // The request watch can observe `needs-interaction` more than
                // once (initial read + the StatusChanged signal); only open the
                // browser the first time. "Open again" stays manual.
                let already_open = pending.url.is_some();
                pending.url = Some(url.clone());
                if !already_open {
                    tasks.push(self.update(Message::LaunchUrl(url)));
                }
            }
            Message::OpenAuthUrl => {
                if let Some(url) = self
                    .pending_auth
                    .as_ref()
                    .and_then(|pending| pending.url.clone())
                {
                    tasks.push(self.update(Message::LaunchUrl(url)));
                }
            }
            Message::CancelAuth => {
                if let (Some(client), Some(path)) = (
                    self.client.clone(),
                    self.pending_auth
                        .as_ref()
                        .and_then(|pending| pending.request_path.clone()),
                ) {
                    tasks.push(Task::perform(
                        async move {
                            if let Ok(proxy) = client.request_proxy(path).await {
                                let _ = proxy.cancel().await;
                            }
                        },
                        |_| cosmic::action::none(),
                    ));
                }
                self.pending_auth = None;
                tasks.push(self.update(Message::CloseDialog));
            }
            Message::AuthFailed(error) => {
                self.pending_auth = None;
                tasks.push(self.update(Message::CloseDialog));
                tasks.push(self.update(Message::ShowToast(fl!(
                    "auth-failed",
                    error = error.as_str()
                ))));
            }
            Message::AccountAdded(id) => {
                self.pending_auth = None;
                tasks.push(self.update(Message::CloseDialog));

                if let Some(client) = self.client.clone() {
                    tasks.push(Task::perform(
                        async move { client.get_account(&id).await },
                        |account| match account {
                            Ok(account) => cosmic::action::app(Message::AccountReady(account)),
                            Err(err) => {
                                tracing::error!("Failed to load the new account: {err}");
                                cosmic::action::none()
                            }
                        },
                    ));
                }
                tasks.push(self.update(Message::LoadAccounts));
            }
            Message::AccountReady(account) => {
                let label = self.account_label(&account);
                tasks.push(self.update(Message::SelectAccount(account)));
                tasks.push(self.update(Message::ShowToast(fl!(
                    "account-added",
                    account = label.as_str()
                ))));
            }
        }

        Task::batch(tasks)
    }
}

impl AppModel {
    /// The name to show for an account wherever a single line has to identify it.
    fn account_label(&self, account: &Account) -> String {
        if account.username.is_empty() {
            account.display_name.clone()
        } else {
            account.username.clone()
        }
    }

    fn rebuild_nav(&mut self) {
        let selected_id = self.selected_account.as_ref().map(|account| account.id);
        let entries = self
            .accounts
            .iter()
            .map(|account| {
                (
                    self.account_label(account),
                    self.provider_icon_handle(&account.provider),
                    account.clone(),
                )
            })
            .collect::<Vec<_>>();

        self.nav.clear();
        for (label, icon, account) in entries {
            let is_selected = selected_id == Some(account.id);
            let mut entry = self.nav.insert();
            if is_selected {
                entry = entry.activate();
            }
            entry.icon(icon.icon()).text(label).data(account);
        }
    }

    fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        match self.core.main_window_id() {
            Some(id) => self.set_window_title(window_title, id),
            None => Task::none(),
        }
    }
}

const ACCOUNT_ADDED: &str = "account_added";
const ACCOUNT_REMOVED: &str = "account_removed";

const DAEMON_SIGNALS: [&str; 2] = [ACCOUNT_ADDED, ACCOUNT_REMOVED];

/// Bridges one of the daemon's `Manager` D-Bus signals into the application's message stream.
async fn watch_daemon_signal(signal: &'static str, output: Sender<Message>) {
    let Ok(client) = AccountsClient::new().await else {
        tracing::error!("Failed to connect to the daemon to watch for {signal}");
        return;
    };

    match signal {
        ACCOUNT_ADDED => match client.receive_account_added().await {
            Ok(stream) => {
                forward(stream, output, |signal| {
                    let args = signal.args().ok()?;
                    account_id_from_path(args.account()).map(Message::AccountAdded)
                })
                .await;
            }
            Err(err) => tracing::error!("Failed to watch for {signal}: {err}"),
        },
        ACCOUNT_REMOVED => match client.receive_account_removed().await {
            Ok(stream) => forward(stream, output, |_| Some(Message::LoadAccounts)).await,
            Err(err) => tracing::error!("Failed to watch for {signal}: {err}"),
        },
        unknown => tracing::error!("Unknown daemon signal: {unknown}"),
    }
}

/// Drives a single sign-in attempt: watches its `Request` object until it reaches a
/// terminal status, emitting `AuthUrlReady`/`AccountAdded`/`AuthFailed` along the way.
async fn watch_request(path: zbus::zvariant::OwnedObjectPath, mut output: Sender<Message>) {
    let Ok(client) = AccountsClient::new().await else {
        tracing::error!("Failed to connect to the daemon to watch the sign-in request");
        return;
    };
    let Ok(proxy) = client.request_proxy(path).await else {
        tracing::error!("Failed to reach the sign-in request object");
        return;
    };

    if emit_request_state(&proxy, &mut output).await.is_break() {
        return;
    }

    let Ok(mut changes) = proxy.receive_request_status_changed().await else {
        tracing::error!("Failed to watch the sign-in request for status changes");
        return;
    };

    while changes.next().await.is_some() {
        if emit_request_state(&proxy, &mut output).await.is_break() {
            break;
        }
    }
}

async fn emit_request_state(
    proxy: &accounts_core::proxy::RequestProxy<'static>,
    output: &mut Sender<Message>,
) -> std::ops::ControlFlow<()> {
    let status = proxy.status().await.unwrap_or_default();
    match status.as_str() {
        "needs-interaction" => {
            if let Ok(uri) = proxy.interaction_uri().await
                && !uri.is_empty()
            {
                let _ = output.send(Message::AuthUrlReady(uri)).await;
            }
            std::ops::ControlFlow::Continue(())
        }
        "succeeded" => {
            if let Ok(account_path) = proxy.account().await
                && let Some(id) = account_id_from_path(&account_path)
            {
                let _ = output.send(Message::AccountAdded(id)).await;
            }
            std::ops::ControlFlow::Break(())
        }
        "failed" => {
            let reason = proxy.error_message().await.unwrap_or_default();
            let _ = output.send(Message::AuthFailed(reason)).await;
            std::ops::ControlFlow::Break(())
        }
        "cancelled" => std::ops::ControlFlow::Break(()),
        _ => std::ops::ControlFlow::Continue(()),
    }
}

/// The path is `/dev/edfloreshz/Accounts/Accounts/<uuid with `-` replaced by `_`>`.
fn account_id_from_path(path: &zbus::zvariant::ObjectPath<'_>) -> Option<Uuid> {
    let segment = path.as_str().rsplit('/').next()?;
    Uuid::parse_str(&segment.replace('_', "-")).ok()
}

async fn forward<S, T>(
    mut signals: S,
    mut output: Sender<Message>,
    into_message: impl Fn(T) -> Option<Message>,
) where
    S: Stream<Item = T> + Unpin,
{
    while let Some(signal) = signals.next().await {
        let Some(message) = into_message(signal) else {
            continue;
        };
        if let Err(err) = output.send(message).await {
            tracing::warn!("Failed to forward a daemon signal: {err}");
            return;
        }
    }
}

fn key_binds() -> HashMap<menu::KeyBind, MenuAction> {
    use menu::key_bind::Modifier;

    HashMap::from([(
        menu::KeyBind {
            modifiers: vec![Modifier::Ctrl],
            key: Key::Character("n".into()),
        },
        MenuAction::AddAccount,
    )])
}

fn about() -> widget::about::About {
    widget::about::About::default()
        .name(fl!("app-title"))
        .icon(widget::icon::from_svg_bytes(APP_ICON))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Eduardo Flores")
        .comments(fl!("manage-online"))
        .license("GPL-3.0-only")
        .links([
            (fl!("repository"), REPOSITORY.to_string()),
            (
                fl!("git-description", hash = short_hash(), date = commit_date()),
                format!("{REPOSITORY}/commits/{}", env!("VERGEN_GIT_SHA")),
            ),
        ])
}

fn short_hash() -> String {
    env!("VERGEN_GIT_SHA").chars().take(7).collect()
}

fn commit_date() -> &'static str {
    env!("VERGEN_GIT_COMMIT_DATE")
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
    AddAccount,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
            MenuAction::AddAccount => Message::OpenDialog(DialogPage::AddAccount),
        }
    }
}
