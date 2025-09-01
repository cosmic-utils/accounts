use cosmic_accounts::{CosmicAccounts, Result};
use std::future::pending;
use tracing::info;
use tracing_subscriber;
use zbus::connection;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting COSMIC Accounts daemon...");

    // Create the accounts service
    let mut accounts = CosmicAccounts::new()?;

    // Setup providers (this would load from config in production)
    accounts.setup_providers().await?;

    info!("Setting up D-Bus connection...");

    // Create D-Bus connection and serve the interface
    let _conn = connection::Builder::session()?
        .name("com.system76.CosmicAccounts")?
        .serve_at("/com/system76/CosmicAccounts", accounts)?
        .build()
        .await?;

    info!("COSMIC Accounts daemon started successfully");
    info!("Listening on D-Bus interface: com.system76.CosmicAccounts");
    info!("Object path: /com/system76/CosmicAccounts");

    // Keep the service running
    pending::<()>().await;

    Ok(())
}
