// SPDX-License-Identifier: GPL-3.0-only

use crate::fl;
use accounts::models::{Account, Provider, Service};
use accounts::{AccountsClient, Local, Uuid, zbus};
use cosmic::app::context_drawer;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription, stream};
use cosmic::prelude::*;
use cosmic::theme::spacing;
use cosmic::widget::image::Handle;
use cosmic::widget::{self, ToastId, menu, nav_bar};
use cosmic::{cosmic_theme, theme};
use futures_util::{SinkExt, StreamExt};
use std::collections::{HashMap, VecDeque};

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Dialog pages for the application.
    dialog_pages: VecDeque<DialogPage>,
    /// Toasts for the application.
    toasts: widget::Toasts<Message>,
    /// Client for interacting with the Accounts for COSMIC API.
    client: Option<AccountsClient>,
    // Accounts data.
    accounts: Vec<Account>,
    // Providers list.
    providers: Vec<Provider>,
    selected_account: Option<Account>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    // COSMIC
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    ToggleDialog(DialogPage),
    #[allow(unused)]
    UpdateDialog(DialogPage),
    CloseDialog,
    LaunchUrl(String),
    ShowToast(String),
    CloseToast(ToastId),
    // Accounts
    LoadAccounts,
    AddAccount(Uuid),
    DeleteAccount(Uuid),
    RemoveAccount(Uuid),
    ToggleService(Service, bool),
    EnableAccount(bool),
    AccountSelected(Account),
    SetAccounts(Vec<Account>),
    AccountExists,
    // Client
    CreateClient,
    SetClient(Option<AccountsClient>),
    // Auth
    StartAuth(Provider),
}

impl<'a> AppModel {
    fn welcome_view(&self) -> impl Into<Element<'_, Message>> {
        // Main container
        let mut main_column = widget::column()
            .spacing(spacing().space_m)
            .padding([spacing().space_m, spacing().space_xs])
            .align_x(Alignment::Center);

        // App icon and title section
        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON))
            .width(64)
            .height(64);

        let title = widget::text::title1(fl!("app-title")).align_x(Horizontal::Center);

        let subtitle = widget::text(fl!("manage-online"))
            .size(16)
            .align_x(Horizontal::Center)
            .class(cosmic::style::Text::Accent);

        let header_section = widget::column()
            .spacing(spacing().space_xs)
            .align_x(Alignment::Center)
            .push(icon)
            .push(title)
            .push(subtitle);

        main_column = main_column.push(header_section);

        // Welcome message
        let welcome_text = widget::text(fl!("connect-accounts"))
            .align_x(Horizontal::Center)
            .class(cosmic::theme::Text::Default);

        main_column = main_column.push(welcome_text);

        // Providers section
        if !self.providers.is_empty() {
            let mut providers_row = widget::row().spacing(spacing().space_s);
            let mut current_row_count = 0;
            let max_per_row = 3;
            let mut providers_column = widget::column().spacing(spacing().space_xs);

            for provider in &self.providers {
                // Add provider icon if available
                let provider_button = widget::row()
                    .spacing(spacing().space_xxs)
                    .padding(spacing().space_m)
                    .align_y(Alignment::Center)
                    .push(
                        widget::image(Self::provider_icon(provider))
                            .width(24)
                            .height(24),
                    )
                    .push(widget::text(provider.to_string()))
                    .apply(widget::button::custom)
                    .on_press(Message::StartAuth(provider.clone()));

                providers_row = providers_row.push(provider_button);
                current_row_count += 1;

                if current_row_count >= max_per_row {
                    providers_column = providers_column
                        .push(widget::container(providers_row).center_x(Length::Fill));
                    providers_row = widget::row().spacing(spacing().space_s);
                    current_row_count = 0;
                }
            }

            // Add any remaining providers in the last row
            if current_row_count > 0 {
                providers_column =
                    providers_column.push(widget::container(providers_row).center_x(Length::Fill));
            }

            main_column = main_column.push(providers_column);
        } else {
            // No providers available message
            let no_providers_text = widget::text(fl!("no-account-providers"))
                .align_x(Horizontal::Center)
                .class(cosmic::theme::Text::Default);

            main_column = main_column.push(no_providers_text);
        }
        // Call to action
        let cta_text = widget::text(fl!("add-account-body"))
            .size(14)
            .align_x(Horizontal::Center)
            .class(cosmic::theme::Text::Default);

        main_column = main_column.push(cta_text);

        // Wrap in a container with proper centering
        widget::container(main_column)
            .center_x(Length::Fill)
            .width(Length::Fill)
    }

    fn add_account_dialog() -> impl Into<Element<'a, Message>> {
        // Main container
        let mut main_column = widget::column()
            .spacing(spacing().space_m)
            .padding([spacing().space_m, spacing().space_xs])
            .align_x(Alignment::Center);

        // App icon and title section

        // Providers section
        if !Provider::list().is_empty() {
            let mut providers_row = widget::row().spacing(spacing().space_s);
            let mut current_row_count = 0;
            let max_per_row = 3;
            let mut providers_column = widget::column().spacing(spacing().space_xs);

            for provider in &Provider::list() {
                // Add provider icon if available
                let provider_button = widget::row()
                    .spacing(spacing().space_xxs)
                    .padding(spacing().space_m)
                    .align_y(Alignment::Center)
                    .push(
                        widget::image(Self::provider_icon(provider))
                            .width(24)
                            .height(24),
                    )
                    .push(widget::text(provider.to_string()))
                    .apply(widget::button::custom)
                    .on_press(Message::StartAuth(provider.clone()));

                providers_row = providers_row.push(provider_button);
                current_row_count += 1;

                if current_row_count >= max_per_row {
                    providers_column = providers_column
                        .push(widget::container(providers_row).center_x(Length::Fill));
                    providers_row = widget::row().spacing(spacing().space_s);
                    current_row_count = 0;
                }
            }

            // Add any remaining providers in the last row
            if current_row_count > 0 {
                providers_column =
                    providers_column.push(widget::container(providers_row).center_x(Length::Fill));
            }

            main_column = main_column.push(providers_column);
        } else {
            // No providers available message
            let no_providers_text = widget::text("No account providers are currently available.")
                .align_x(Horizontal::Center)
                .class(cosmic::theme::Text::Default);

            main_column = main_column.push(no_providers_text);
        }

        // Wrap in a container with proper centering
        widget::container(main_column)
            .center_x(Length::Fill)
            .width(Length::Fill)
    }

    fn account_view(&self) -> impl Into<Element<'_, Message>> {
        let Some(account) = &self.selected_account else {
            return widget::column().spacing(spacing().space_xs);
        };

        let provider_header = widget::row()
            .push(widget::image(Self::provider_icon(&account.provider)).width(60))
            .push(
                widget::column()
                    .push(widget::text::title1(account.provider.to_string()))
                    .push(widget::text::caption_heading(account.username.to_string())),
            )
            .spacing(spacing().space_xs)
            .align_y(Vertical::Center);

        let account_state =
            widget::settings::section()
                .title(fl!("account"))
                .add(widget::settings::flex_item(
                    fl!("enabled"),
                    widget::toggler(account.enabled).on_toggle(Message::EnableAccount),
                ));

        let account_details = widget::settings::section()
            .title(fl!("details"))
            .add(widget::settings::flex_item(
                fl!("provider"),
                widget::text::body(account.provider.to_string()),
            ))
            .add(widget::settings::flex_item(
                fl!("display-name"),
                widget::text::body(&account.display_name),
            ))
            .add(widget::settings::flex_item(
                fl!("email"),
                widget::text::body(account.email.clone().unwrap_or(fl!("no-email"))),
            ))
            .add(widget::settings::flex_item(
                fl!("created-at"),
                widget::text::body(
                    account
                        .created_at
                        .with_timezone(&Local)
                        .format("%B %d, %Y at %I:%M %p")
                        .to_string(),
                ),
            ))
            .add(widget::settings::flex_item(
                fl!("last-used"),
                widget::text::body(
                    account
                        .last_used
                        .map(|last_used| {
                            last_used
                                .with_timezone(&Local)
                                .format("%B %d, %Y at %I:%M %p")
                                .to_string()
                        })
                        .unwrap_or(fl!("no-usage")),
                ),
            ));

        let mut services = widget::settings::section().title(fl!("services"));
        for (service, enabled) in &account.services {
            services = services.add(widget::settings::item(
                service.to_string(),
                widget::toggler(*enabled)
                    .on_toggle(|enabled| Message::ToggleService(service.clone(), enabled)),
            ));
        }

        widget::column()
            .push(provider_header)
            .push(account_state)
            .push(account_details)
            .push(services)
            .spacing(spacing().space_xxs)
    }

    fn provider_icon(provider: &Provider) -> Handle {
        match provider {
            Provider::Google => {
                Handle::from_bytes(include_bytes!("../resources/img/google.png").to_vec())
            }
            Provider::Microsoft => {
                Handle::from_bytes(include_bytes!("../resources/img/microsoft.png").to_vec())
            }
        }
    }
}

/// Create a COSMIC application from the app model
impl<'a> cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "dev.edfloreshz.Accounts";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav: nav_bar::Model::default(),
            key_binds: HashMap::new(),
            toasts: widget::toaster::Toasts::new(Message::CloseToast),
            dialog_pages: VecDeque::new(),
            client: None,
            accounts: Vec::new(),
            providers: Provider::list().to_vec(),
            selected_account: None,
        };

        let tasks = vec![
            app.update_title(),
            cosmic::task::message(Message::CreateClient),
        ];

        (app, Task::batch(tasks))
    }

    /// Elements to pack at the start of the header bar.
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
        ]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        let dialog_page = self.dialog_pages.front()?;
        let dialog = dialog_page.view();
        Some(dialog.into())
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        // Activate the page in the model.
        self.nav.activate(id);

        let mut tasks = vec![self.update_title()];
        let account = self.nav.active_data::<Account>();
        if let Some(account) = account {
            tasks.push(self.update(Message::AccountSelected(account.clone())));
        }
        Task::batch(tasks)
    }

    fn on_escape(&mut self) -> cosmic::app::Task<Self::Message> {
        if self.dialog_pages.pop_front().is_some() {
            return Task::none();
        }

        self.core.window.show_context = false;

        Task::none()
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::context_drawer(
                self.about(),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(fl!("about")),
        })
    }

    fn footer(&self) -> Option<Element<'_, Self::Message>> {
        self.selected_account.as_ref().map(|account| {
            widget::row()
                .push(widget::horizontal_space())
                .push(
                    widget::button::standard(fl!("remove"))
                        .class(cosmic::style::Button::Destructive)
                        .on_press(Message::DeleteAccount(account.id)),
                )
                .spacing(spacing().space_xxs)
                .apply(widget::container)
                .class(cosmic::style::Container::Card)
                .padding(spacing().space_xxs)
                .into()
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        let content = if self.selected_account.is_some() {
            self.account_view().into()
        } else {
            self.welcome_view().into()
        };

        let toaster =
            widget::row::row().push(widget::toaster(&self.toasts, widget::horizontal_space()));

        widget::column()
            .push(widget::scrollable(content))
            .push(toaster)
            .padding(spacing().space_xxs)
            .height(Length::Fill)
            .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        let Some(client) = self.client.clone() else {
            return Subscription::none();
        };
        let account_changed_client = client.clone();
        let account_removed_client = client.clone();
        let account_exists_client = client.clone();

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
            Subscription::run_with_id(
                "account_added",
                stream::channel(1, move |mut output| async move {
                    if let Ok(mut account_added_stream) = client.receive_account_added().await {
                        while let Some(account_added) = account_added_stream.next().await {
                            let args = account_added.args().expect("Error parsing arguments");
                            if let Err(err) = output
                                .send(Message::AddAccount(
                                    Uuid::parse_str(args.account_id())
                                        .expect("Expected account id to be UUID"),
                                ))
                                .await
                            {
                                tracing::warn!("failed to send message from subscription: {}", err);
                            }
                        }
                    }
                }),
            ),
            Subscription::run_with_id(
                "account_changed",
                stream::channel(1, move |mut output| async move {
                    if let Ok(mut account_changed_stream) =
                        account_changed_client.receive_account_changed().await
                    {
                        while let Some(_) = account_changed_stream.next().await {
                            if let Err(err) = output.send(Message::LoadAccounts).await {
                                tracing::warn!("failed to send message from subscription: {}", err);
                            }
                        }
                    }
                }),
            ),
            Subscription::run_with_id(
                "account_removed",
                stream::channel(1, move |mut output| async move {
                    if let Ok(mut account_removed_stream) =
                        account_removed_client.receive_account_removed().await
                    {
                        while let Some(_) = account_removed_stream.next().await {
                            if let Err(err) = output.send(Message::LoadAccounts).await {
                                tracing::warn!("failed to send message from subscription: {}", err);
                            }
                        }
                    }
                }),
            ),
            Subscription::run_with_id(
                "account_exists",
                stream::channel(1, move |mut output| async move {
                    if let Ok(mut account_exists_stream) =
                        account_exists_client.receive_account_exists().await
                    {
                        while let Some(_) = account_exists_stream.next().await {
                            if let Err(err) = output.send(Message::AccountExists).await {
                                tracing::warn!("failed to send message from subscription: {}", err);
                            }
                        }
                    }
                }),
            ),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        let mut tasks = vec![];

        match message {
            Message::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            }
            Message::SubscriptionChannel => {
                // For example purposes only.
            }
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }
            Message::ToggleDialog(page) => self.dialog_pages.push_back(page),
            Message::UpdateDialog(page) => {
                self.dialog_pages[0] = page.clone();
            }
            Message::CloseDialog => {
                self.dialog_pages.pop_front();
            }
            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },
            Message::ShowToast(message) => {
                tasks.push(
                    self.toasts
                        .push(widget::toaster::Toast::new(message))
                        .map(cosmic::Action::App),
                );
            }
            Message::CloseToast(id) => self.toasts.remove(id),
            Message::LoadAccounts => {
                let client = self.client.clone();
                if let Some(client) = client {
                    tasks.push(Task::perform(
                        async move { client.list_accounts().await },
                        |accounts| match accounts {
                            Ok(accounts) => cosmic::Action::App(Message::SetAccounts(accounts)),
                            Err(err) => {
                                tracing::error!("{err}");
                                cosmic::Action::None
                            }
                        },
                    ));
                }
            }
            Message::EnableAccount(enable) => {
                if let (Some(mut client), Some(account)) =
                    (self.client.clone(), self.selected_account.clone())
                {
                    tasks.push(Task::perform(
                        async move {
                            client.set_account_enabled(&account.id, enable).await?;
                            Ok(())
                        },
                        |result: Result<(), zbus::fdo::Error>| match result {
                            Ok(_) => cosmic::action::app(Message::LoadAccounts),
                            Err(err) => {
                                tracing::error!("Failed to toggle account: {}", err);
                                cosmic::action::none()
                            }
                        },
                    ));
                }
            }
            Message::ToggleService(service, enabled) => {
                if let (Some(mut client), Some(account)) =
                    (self.client.clone(), self.selected_account.clone())
                {
                    tasks.push(Task::perform(
                        async move {
                            client
                                .set_service_enabled(&account.id, &service, enabled)
                                .await?;
                            Ok(())
                        },
                        |result: Result<(), zbus::fdo::Error>| match result {
                            Ok(_) => cosmic::action::app(Message::LoadAccounts),
                            Err(err) => {
                                tracing::error!("Failed to set service: {}", err);
                                cosmic::action::none()
                            }
                        },
                    ));
                }
            }
            Message::AddAccount(id) => {
                let client = self.client.clone();
                if let Some(client) = client {
                    tasks.push(Task::perform(
                        async move { client.get_account(&id.to_string()).await },
                        |account| match account {
                            Ok(account) => cosmic::action::app(Message::AccountSelected(account)),
                            Err(err) => {
                                tracing::error!("{err}");
                                cosmic::action::none()
                            }
                        },
                    ));
                }
                tasks.push(self.update(Message::CloseDialog));
                tasks.push(self.update(Message::LoadAccounts));
            }
            Message::DeleteAccount(account_id) => {
                tracing::info!("Removing account: {}", account_id);
                if let Some(mut client) = self.client.clone() {
                    tasks.push(Task::perform(
                        async move {
                            client.remove_account(&account_id).await?;
                            client.account_removed(&account_id).await?;
                            Ok(account_id)
                        },
                        |result: Result<Uuid, zbus::fdo::Error>| match result {
                            Ok(account_id) => {
                                cosmic::action::app(Message::RemoveAccount(account_id.clone()))
                            }
                            Err(err) => {
                                tracing::error!("Failed to remove account: {}", err);
                                cosmic::action::none()
                            }
                        },
                    ));
                }
            }
            Message::RemoveAccount(account_id) => {
                self.accounts.retain(|account| account.id != account_id);
                self.selected_account = None;
            }
            Message::AccountExists => {
                tasks.push(self.update(Message::ShowToast(fl!("account-exists"))));
            }
            Message::AccountSelected(account) => self.selected_account = Some(account),
            Message::SetAccounts(accounts) => {
                self.core.nav_bar_set_toggled(!accounts.is_empty());
                self.accounts.clear();
                self.nav.clear();

                self.accounts = accounts;
                if let Some(selected) = self.selected_account.clone()
                    && let Some(account) = self.accounts.iter().find(|a| a.id == selected.id)
                {
                    self.selected_account = Some(account.clone());
                    for account in &self.accounts {
                        let account = account.clone();

                        if account.id == selected.id {
                            self.nav
                                .insert()
                                .activate()
                                .text(account.username.clone())
                                .data(account);
                        } else {
                            self.nav
                                .insert()
                                .text(account.username.clone())
                                .data(account);
                        }
                    }
                } else {
                    for account in &self.accounts {
                        let account = account.clone();

                        self.nav
                            .insert()
                            .text(account.username.clone())
                            .data(account);
                    }
                }
            }
            Message::CreateClient => {
                tasks.push(Task::perform(
                    async {
                        match AccountsClient::new().await {
                            Ok(client) => Some(client),
                            Err(err) => {
                                tracing::error!("{err}");
                                None
                            }
                        }
                    },
                    |client| cosmic::Action::App(Message::SetClient(client)),
                ));
            }
            Message::SetClient(client) => {
                self.client = client;
                tasks.push(cosmic::task::message(Message::LoadAccounts));
            }
            Message::StartAuth(provider) => {
                tracing::info!(
                    "Starting authentication for provider: {}",
                    provider.to_string()
                );

                let Some(mut client) = self.client.clone() else {
                    tracing::error!("No client available");
                    return Task::none();
                };

                tasks.push(Task::perform(
                    async move {
                        let url = client.start_authentication(&provider).await?;
                        open::that_detached(url)
                            .map_err(|e| zbus::Error::Failure(e.to_string()))?;
                        Ok(())
                    },
                    |result: Result<(), zbus::Error>| match result {
                        Ok(_) => cosmic::action::none(),
                        Err(err) => {
                            tracing::error!("Failed to start authentication: {}", err);
                            cosmic::action::none()
                        }
                    },
                ));
            }
        }
        Task::batch(tasks)
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

        let title = widget::text::title3(fl!("app-title"));

        let hash = env!("VERGEN_GIT_SHA");
        let short_hash: String = hash.chars().take(7).collect();
        let date = env!("VERGEN_GIT_COMMIT_DATE");

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::OpenRepositoryUrl)
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .push(
                widget::button::link(fl!(
                    "git-description",
                    hash = short_hash.as_str(),
                    date = date
                ))
                .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
                .padding(0),
            )
            .align_x(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

/// The context page to display in the context drawer.
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
            MenuAction::AddAccount => Message::ToggleDialog(DialogPage::AddAccount),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DialogPage {
    AddAccount,
}

impl<'a> DialogPage {
    fn view(&self) -> impl Into<Element<'_, Message>> {
        match self {
            DialogPage::AddAccount => widget::dialog()
                .title(fl!("add-account-title"))
                .body(fl!("add-account-body"))
                .primary_action(widget::button::text(fl!("close")).on_press(Message::CloseDialog))
                .control(AppModel::add_account_dialog()),
        }
    }
}
