// SPDX-License-Identifier: GPL-3.0-only

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod daemon;
mod i18n;

fn main() -> cosmic::iced::Result {
    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var("RUST_LOG", "accounts_ui=info");
        }
    }
    tracing_subscriber::registry()
        .with(EnvFilter::from_env("RUST_LOG"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    if std::env::var("ACCOUNTS_HEADLESS").is_ok() {
        let runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        runtime
            .block_on(daemon::run())
            .expect("daemon exited with an error");
        return Ok(());
    }

    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    i18n::init(&requested_languages);

    let settings = cosmic::app::Settings::default()
        .size(cosmic::iced::Size::new(920.0, 640.0))
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(400.0)
                .min_height(360.0),
        );

    cosmic::app::run::<app::AppModel>(settings, ())
}
