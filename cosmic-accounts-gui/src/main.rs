use cosmic::app::Task;
use cosmic::iced::{Alignment, Length};
use cosmic::{widget, Application, Core, Element};
use cosmic_accounts::{CosmicAccountsProxy, Provider};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use zbus::Connection;

const APP_ID: &str = "com.system76.CosmicAccounts";

#[derive(Debug, Clone)]
pub enum Message {
    AddAccount,
    RemoveAccount(String),
    CreateClient,
    SetClient(Option<CosmicAccountsProxy<'static>>),
    LoadAccounts(Vec<AccountInfo>),
    RefreshAccounts,
    AccountSelected(String),
    ProviderSelected(Provider),
    StartAuth,
}

pub struct CosmicAccountsApp {
    core: Core,
    client: Option<CosmicAccountsProxy<'static>>,
    accounts: Vec<AccountInfo>,
    selected_account: Option<String>,
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

        (
            app,
            Task::perform(load_accounts(), |accounts| {
                cosmic::Action::App(Message::LoadAccounts(accounts))
            }),
        )
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        let mut tasks = vec![];

        match message {
            Message::AddAccount => {
                self.show_add_dialog = true;
            }
            Message::RemoveAccount(account_id) => {
                info!("Removing account: {}", account_id);
                self.accounts.retain(|account| account.id != account_id);
                if self.selected_account.as_ref() == Some(&account_id) {
                    self.selected_account = None;
                }
            }
            Message::CreateClient => {
                tasks.push(Task::perform(
                    async {
                        match Connection::session().await {
                            Ok(connection) => match CosmicAccountsProxy::new(&connection).await {
                                Ok(proxy) => Some(proxy),
                                Err(err) => {
                                    tracing::error!("{err}");
                                    None
                                }
                            },
                            Err(err) => {
                                tracing::error!("{err}");
                                None
                            }
                        }
                    },
                    |client| cosmic::Action::App(Message::SetClient(client)),
                ));
            }
            Message::SetClient(client) => self.client = client,
            Message::LoadAccounts(accounts) => {
                info!("Refreshing accounts list");
                println!("Loaded accounts: {:?}", accounts);
            }
            Message::RefreshAccounts => {
                info!("Refreshing accounts list");
                tasks.push(Task::perform(load_accounts(), |_| {
                    // For now, we'll just return None as we don't have actual accounts
                    cosmic::Action::None
                }));
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
                                match client.start_authentication(&provider.to_string()).await {
                                    Ok(url) => match open::that(url) {
                                        Ok(_) => {
                                            tracing::info!("Opened URL")
                                        }
                                        Err(_) => {}
                                    },
                                    Err(err) => {
                                        tracing::error!("{err}");
                                    }
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

    fn view(&self) -> Element<Self::Message> {
        let content = if self.accounts.is_empty() {
            self.view_empty_state()
        } else {
            self.view_accounts_list()
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
}

impl CosmicAccountsApp {
    fn view_empty_state(&self) -> Element<Message> {
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
            .into()
    }

    fn view_accounts_list(&self) -> Element<Message> {
        let mut account_items = Vec::new();

        for account in &self.accounts {
            let account_row = widget::row()
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
                        .on_press(Message::RemoveAccount(account.id.clone())),
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
            .into()
    }

    fn view_add_dialog(&self) -> Element<Message> {
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
            .into()
    }
}

async fn load_accounts() -> Vec<AccountInfo> {
    // In a real implementation, this would connect to the D-Bus service
    // and load the actual accounts
    info!("Loading accounts from D-Bus service");

    // For now, return empty list
    Vec::new()
}

fn main() -> cosmic::iced::Result {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
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
