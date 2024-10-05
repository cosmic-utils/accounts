use cosmic_accounts::CosmicAccounts;
use std::{error::Error, future::pending};
use zbus::connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let accounts = CosmicAccounts::new();
    let _conn = connection::Builder::session()?
        .name("com.system76.CosmicAccounts")?
        .serve_at("/com/system76/CosmicAccounts", accounts)?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
