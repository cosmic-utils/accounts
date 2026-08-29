// SPDX-License-Identifier: GPL-3.0-only

#[tokio::main]
async fn main() -> zbus::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "accounts_example_handler=info") };
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    accounts_example_handler::run().await
}
