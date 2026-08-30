// SPDX-License-Identifier: GPL-3.0-only

//! `dev.edfloreshz.Accounts.ConsentPrompt` — the consent dialog for
//! `Credentials.GetAccessToken`.
//!
//! It runs as a **separate process** from the credentials daemon (a security
//! boundary: the daemon stays headless and toolkit-agnostic). It is its own
//! binary, D-Bus-activated via the `accounts-consent-prompt` wrapper; the
//! `accounts_ui` binary also re-enters this code when `ACCOUNTS_CONSENT_PROMPT`
//! is set.
//!
//! Single-shot: serves exactly one `Prompt` call, shows the dialog, returns the
//! decision, and exits. The bus re-activates it for the next prompt.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use cosmic::prelude::*;
use cosmic::widget;
use tokio::sync::oneshot;

const DBUS_NAME: &str = "dev.edfloreshz.Accounts.ConsentPrompt";
const OBJECT_PATH: &str = "/dev/edfloreshz/Accounts/ConsentPrompt";
/// Upper bound on how long the process waits for a `Prompt` call before giving
/// up — comfortably past the daemon's own 120s consent timeout.
const IDLE_TIMEOUT: Duration = Duration::from_secs(130);

struct PromptParams {
    caller_name: String,
    account_name: String,
    provider_id: String,
    service: String,
}

struct Job {
    params: PromptParams,
    reply: oneshot::Sender<bool>,
}

type JobSlot = Arc<Mutex<Option<Job>>>;

struct PromptService {
    slot: JobSlot,
}

#[zbus::interface(name = "dev.edfloreshz.Accounts.ConsentPrompt")]
impl PromptService {
    /// Blocks until the user answers. `true` = allow.
    async fn prompt(
        &self,
        caller_name: &str,
        account_name: &str,
        provider_id: &str,
        service: &str,
    ) -> bool {
        let (reply_tx, reply_rx) = oneshot::channel();
        {
            let mut slot = self.slot.lock().expect("job slot poisoned");
            if slot.is_some() {
                // Already showing a prompt; refuse extras rather than queue.
                return false;
            }
            *slot = Some(Job {
                params: PromptParams {
                    caller_name: caller_name.to_string(),
                    account_name: account_name.to_string(),
                    provider_id: provider_id.to_string(),
                    service: service.to_string(),
                },
                reply: reply_tx,
            });
        }

        let decision = reply_rx.await.unwrap_or(false);

        // Reply is about to be sent; exit shortly after so it flushes first.
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(400)).await;
            std::process::exit(0);
        });

        decision
    }
}

/// Entry point for the consent prompt: waits for one `Prompt` call, shows the
/// dialog, returns the decision, and exits.
pub fn run() -> cosmic::iced::Result {
    let slot: JobSlot = Arc::new(Mutex::new(None));

    {
        let slot = slot.clone();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("consent prompt tokio runtime");
            runtime.block_on(async move {
                let built = zbus::connection::Builder::session()
                    .and_then(|b| b.name(DBUS_NAME))
                    .and_then(|b| b.serve_at(OBJECT_PATH, PromptService { slot }))
                    .expect("consent prompt bus setup")
                    .build()
                    .await;
                match built {
                    Ok(_connection) => std::future::pending::<()>().await,
                    Err(e) => {
                        tracing::error!("consent prompt could not own {DBUS_NAME}: {e}");
                        std::process::exit(1);
                    }
                }
            });
        });
    }

    // Wait for the daemon's Prompt call to land a job.
    let started = std::time::Instant::now();
    let job = loop {
        if let Some(job) = slot.lock().expect("job slot poisoned").take() {
            break job;
        }
        if started.elapsed() > IDLE_TIMEOUT {
            tracing::info!("no consent request received; exiting");
            std::process::exit(0);
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let settings = cosmic::app::Settings::default()
        .size(cosmic::iced::Size::new(460.0, 240.0))
        .resizable(None);
    cosmic::app::run::<Prompt>(settings, job)
}

struct Prompt {
    core: cosmic::Core,
    params: PromptParams,
    reply: Option<oneshot::Sender<bool>>,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Allow,
    Deny,
}

impl cosmic::Application for Prompt {
    type Executor = cosmic::executor::Default;
    type Flags = Job;
    type Message = Message;
    const APP_ID: &'static str = "dev.edfloreshz.Accounts.ConsentPrompt";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(core: cosmic::Core, flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let prompt = Prompt {
            core,
            params: flags.params,
            reply: Some(flags.reply),
        };
        (prompt, Task::none())
    }

    fn view(&self) -> Element<'_, Self::Message> {
        use cosmic::iced::Length;

        let p = &self.params;

        let content = widget::Column::new()
            .spacing(12)
            .width(Length::Fill)
            .push(widget::text::title3(format!(
                "Allow access to \u{201c}{}\u{201d}?",
                p.account_name
            )))
            .push(widget::text::body(format!(
                "{} wants to use this account\u{2019}s {} credentials ({}).",
                p.caller_name, p.service, p.provider_id
            )));

        // Buttons are pinned outside the scroll area so they are always visible,
        // however long the caller/account names are.
        let actions = widget::Row::new()
            .spacing(8)
            .width(Length::Fill)
            .push(widget::space::horizontal())
            .push(widget::button::standard("Deny").on_press(Message::Deny))
            .push(widget::button::suggested("Allow").on_press(Message::Allow));

        widget::Column::new()
            .spacing(16)
            .padding(24)
            .push(
                widget::scrollable(content)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .push(actions)
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        let decision = matches!(message, Message::Allow);
        if let Some(reply) = self.reply.take() {
            let _ = reply.send(decision);
        }
        // The bus side schedules process exit once it has the reply.
        Task::none()
    }
}
