pub mod account;
pub mod auth;
pub mod error;
pub mod services;
pub mod storage;

use accounts_core::{AccountsClient, ProviderRegistry, models::Account};
use tokio::sync::OnceCell;
use tracing::info;

use account::AccountsInterface;
pub use error::{Error, Result};
use services::ServiceFactory;
use zbus::Connection;

pub static CONNECTION: OnceCell<Connection> = OnceCell::const_new();
pub static REGISTRY: OnceCell<ProviderRegistry> = OnceCell::const_new();

pub const REDIRECT_SCHEME: &str = "dev.edfloreshz.accounts";

/// Runs the background D-Bus service.
pub async fn run() -> Result<()> {
    info!("Starting Accounts for COSMIC daemon...");

    REGISTRY
        .set(ProviderRegistry::load_default())
        .expect("registry set once at startup");
    info!(
        "Loaded {} provider manifest(s)",
        REGISTRY.get().unwrap().list().len()
    );

    info!("Setting up D-Bus connection...");
    let service = AccountsInterface::new()
        .await
        .map_err(|e| zbus::Error::Failure(e.to_string()))?;

    let accounts: Vec<Account> = service
        .list_accounts()
        .await
        .into_iter()
        .map(Into::into)
        .collect();

    CONNECTION
        .set(
            zbus::connection::Builder::session()?
                .name("dev.edfloreshz.Accounts")?
                .serve_at("/dev/edfloreshz/Accounts/Account", service)?
                .build()
                .await?,
        )
        .unwrap();

    for account in accounts {
        let services = ServiceFactory::create_services(&account);
        for service in services {
            service.add_service().await?;
        }
    }

    info!("D-Bus service started on: dev.edfloreshz.Accounts");
    info!("Object path: /dev/edfloreshz/Accounts");
    info!("Accounts for COSMIC daemon started successfully");

    std::future::pending::<()>().await;
    Ok(())
}

pub async fn handle_redirect_uri(uri: &str) -> Result<()> {
    let url = url::Url::parse(uri).map_err(|_| Error::InvalidArguments(uri.to_string()))?;

    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_description = None;
    for (key, value) in url.query_pairs() {
        match &*key {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            "error_description" => error_description = Some(value.into_owned()),
            _ => {}
        }
    }

    info!(?code, ?state, ?error, "Received OAuth redirect");

    let Ok(mut client) = AccountsClient::new().await else {
        tracing::error!("Accounts for COSMIC client failed to initialize");
        return Ok(());
    };

    if let Some(error) = error {
        tracing::error!(
            "Authentication failed: {error}: {}",
            error_description.as_deref().unwrap_or("no description")
        );
        return Ok(());
    }

    let (Some(authorization_code), Some(csrf_token)) = (code, state) else {
        tracing::warn!("Redirect URI missing code/state parameters");
        return Ok(());
    };

    match client
        .complete_authentication(&csrf_token, &authorization_code)
        .await
    {
        Ok(account_id) => {
            tracing::info!("User authenticated with ID: {}", account_id);
            if let Err(err) = client.account_added(&account_id).await {
                tracing::error!("Failed to add account: {}", err);
            }
        }
        Err(err) => {
            if err.to_string().to_lowercase().contains("already exists")
                && client.account_exists().await.is_ok()
            {
                tracing::info!("Account already exists");
            }
            tracing::error!("Failed to authenticate user: {}", err);
        }
    }

    Ok(())
}
