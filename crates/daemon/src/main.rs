// SPDX-License-Identifier: GPL-3.0-only

//! Thin wrapper so the credentials daemon can run as a standalone binary. All
//! behaviour lives in `accounts_daemon::run`.

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("accounts_daemon=info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
    runtime
        .block_on(accounts_daemon::run())
        .expect("daemon exited with an error");
}
