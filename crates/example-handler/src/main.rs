// SPDX-License-Identifier: GPL-3.0-only

#[tokio::main]
async fn main() -> zbus::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("accounts_example_handler=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
    accounts_example_handler::run().await
}
