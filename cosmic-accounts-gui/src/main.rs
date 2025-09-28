use cosmic::app::Task;
use cosmic::iced::futures::{SinkExt, StreamExt};
use cosmic::iced::{stream, Alignment, Length, Subscription};
use cosmic::widget::image::Handle;
use cosmic::{widget, Application, Core, Element};
use cosmic_accounts::models::{Account, Provider};
use cosmic_accounts::{CosmicAccountsClient, Uuid};
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const APP_ID: &str = "com.system76.CosmicAccounts";

#[derive(Debug, Clone)]
pub enum Message {
    AddAccount,
    DeleteAccount(Uuid),
    RemoveAccount(Uuid),
    CreateClient,
    SetClient(Option<CosmicAccountsClient>),
    LoadAccounts,
    SetAccounts(Vec<Account>),
    RefreshAccounts,
    AccountSelected(Uuid),
    ProviderSelected(Provider),
    StartAuth,
}

pub struct CosmicAccountsApp {
    core: Core,
    client: Option<CosmicAccountsClient>,
    accounts: Vec<Account>,
    selected_account: Option<Uuid>,
    selected_provider: Option<Provider>,
    show_add_dialog: bool,
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub username: String,
    pub email: Option<String>,
    pub enabled: bool,
    pub capabilities: Vec<String>,
}

impl Application for CosmicAccountsApp {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let app = CosmicAccountsApp {
            core,
            client: None,
            accounts: Vec::new(),
            selected_account: None,
            selected_provider: None,
            show_add_dialog: false,
        };

        (app, cosmic::task::message(Message::CreateClient))
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        let mut tasks = vec![];

        match message {
            Message::AddAccount => {
                self.show_add_dialog = true;
            }
            Message::DeleteAccount(account_id) => {
                info!("Removing account: {}", account_id);
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
                if self.selected_account.as_ref() == Some(&account_id) {
                    self.selected_account = None;
                }
            }
            Message::CreateClient => {
                tasks.push(Task::perform(
                    async {
                        match CosmicAccountsClient::new().await {
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
            Message::SetAccounts(accounts) => {
                self.accounts = accounts;
            }
            Message::RefreshAccounts => {
                tasks.push(cosmic::task::message(Message::LoadAccounts));
            }
            Message::AccountSelected(account_id) => {
                self.selected_account = Some(account_id);
            }
            Message::ProviderSelected(provider) => {
                self.selected_provider = Some(provider);
            }
            Message::StartAuth => {
                if let Some(provider) = &self.selected_provider {
                    info!(
                        "Starting authentication for provider: {}",
                        provider.to_string()
                    );
                    let client = self.client.clone();
                    if let (Some(mut client), Some(provider)) =
                        (client, self.selected_provider.clone())
                    {
                        tasks.push(Task::perform(
                            async move {
                                match client.start_authentication(&provider).await {
                                    Ok(url) => {
                                        if let Err(err) = open::that(url) {
                                            tracing::error!("{err}")
                                        }
                                    }
                                    Err(err) => tracing::error!("{err}"),
                                }
                            },
                            |_| cosmic::Action::None,
                        ));
                    }
                }
                self.show_add_dialog = false;
            }
        }
        Task::batch(tasks)
    }

    fn view<'a>(&'a self) -> Element<'a, Self::Message> {
        let content = if self.accounts.is_empty() {
            self.view_empty_state().into()
        } else {
            self.view_accounts_list().into()
        };

        let main_content = widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20);

        if self.show_add_dialog {
            let dialog = self.view_add_dialog();
            widget::container(
                widget::column()
                    .push(main_content)
                    .push(dialog)
                    .spacing(20)
                    .align_x(Alignment::Center),
            )
            .into()
        } else {
            main_content.into()
        }
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        let Some(client) = self.client.clone() else {
            return Subscription::none();
        };
        let account_changed_client = client.clone();
        let account_removed_client = client.clone();

        Subscription::batch(vec![
            Subscription::run_with_id(
                "account_added",
                stream::channel(1, move |mut output| async move {
                    if let Ok(mut account_added_stream) = client.receive_account_added().await {
                        while let Some(_) = account_added_stream.next().await {
                            if let Err(err) = output.send(Message::LoadAccounts).await {
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
        ])
    }
}

impl CosmicAccountsApp {
    fn view_empty_state<'a>(&'a self) -> impl Into<Element<'a, Message>> {
        widget::column()
            .push(
                widget::text("No accounts configured")
                    .size(24)
                    .align_x(cosmic::iced::alignment::Horizontal::Center),
            )
            .push(
                widget::text("Add an online account to get started")
                    .align_x(cosmic::iced::alignment::Horizontal::Center),
            )
            .push(widget::button::standard("Add Account").on_press(Message::AddAccount))
            .spacing(20)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill)
    }

    fn view_accounts_list<'a>(&'a self) -> impl Into<Element<'a, Message>> {
        let mut account_items = Vec::new();

        for account in &self.accounts {
            let account_row = widget::row()
                .push(widget::image(provider_icon(&account.provider)).width(32))
                .push(
                    widget::column()
                        .push(widget::text(&account.display_name).size(16))
                        .push(
                            widget::text(format!("{} ({})", &account.username, &account.provider))
                                .size(12),
                        )
                        .spacing(4),
                )
                .push(
                    widget::button::destructive("Remove")
                        .on_press(Message::DeleteAccount(account.id.clone())),
                )
                .spacing(10)
                .align_y(Alignment::Center);

            account_items.push(
                widget::container(account_row)
                    .padding(10)
                    .width(Length::Fill)
                    .into(),
            );
        }

        widget::column::with_children(account_items)
            .spacing(10)
            .width(Length::Fill)
    }

    fn view_add_dialog<'a>(&'a self) -> impl Into<Element<'a, Message>> {
        let providers = Provider::list();
        let mut provider_buttons = Vec::new();

        for provider in providers {
            let is_selected = self.selected_provider.as_ref() == Some(&provider);
            let button = if is_selected {
                widget::button::suggested(provider.to_string())
            } else {
                widget::button::standard(provider.to_string())
            };
            provider_buttons.push(
                button
                    .on_press(Message::ProviderSelected(provider.clone()))
                    .into(),
            );
        }

        let dialog_content = widget::column()
            .push(widget::text("Add Account").size(20))
            .push(widget::text("Choose a provider:"))
            .push(widget::row::with_children(provider_buttons).spacing(10))
            .push(
                widget::row()
                    .push(widget::button::standard("Cancel").on_press(Message::AddAccount))
                    .push(
                        widget::button::suggested("Continue")
                            .on_press(Message::StartAuth)
                            .class(if self.selected_provider.is_some() {
                                cosmic::theme::Button::Suggested
                            } else {
                                cosmic::theme::Button::Standard
                            }),
                    )
                    .spacing(10),
            )
            .spacing(20)
            .align_x(Alignment::Center)
            .padding(20);

        widget::container(dialog_content)
            .class(cosmic::theme::Container::Dialog)
            .padding(20)
    }
}

fn main() -> cosmic::iced::Result {
    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var("RUST_LOG", "cosmic_accounts_gui=info");
        }
    }
    tracing_subscriber::registry()
        .with(EnvFilter::from_env("RUST_LOG"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting COSMIC Accounts GUI");

    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(400.0)
            .min_height(300.0),
    );

    cosmic::app::run::<CosmicAccountsApp>(settings, ())
}

fn provider_icon(provider: &Provider) -> Handle {
    match provider {
        Provider::Google => Handle::from_bytes(include_bytes!("../res/img/google.png").to_vec()),
        Provider::Microsoft => {
            Handle::from_bytes(include_bytes!("../res/img/microsoft.png").to_vec())
        }
    }
}
