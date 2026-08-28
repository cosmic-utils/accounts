pub mod account;
pub mod auth;
pub mod error;
pub mod services;
pub mod storage;

use accounts_core::{AccountsClient, ProviderRegistry, models::Account};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::OnceCell;
use tracing::info;

use account::AccountsInterface;
pub use error::{Error, Result};
use services::ServiceFactory;
use zbus::Connection;

pub static CONNECTION: OnceCell<Connection> = OnceCell::const_new();
pub static REGISTRY: OnceCell<ProviderRegistry> = OnceCell::const_new();

const CALLBACK_PORT: u16 = 49173;

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

    let listener = TcpListener::bind(("127.0.0.1", CALLBACK_PORT))
        .await
        .map_err(Error::Io)?;
    info!("OAuth callback listener on http://127.0.0.1:{CALLBACK_PORT}/callback");
    tokio::spawn(run_callback_server(listener));

    info!("Accounts for COSMIC daemon started successfully");

    std::future::pending::<()>().await;
    Ok(())
}

async fn run_callback_server(listener: TcpListener) {
    loop {
        let Ok((mut stream, _)) = listener.accept().await else {
            continue;
        };

        let mut buf = [0u8; 8192];
        let Ok(n) = stream.read(&mut buf).await else {
            continue;
        };

        let request_line = String::from_utf8_lossy(&buf[..n]);
        let query = request_line
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|path| path.split_once('?'))
            .map(|(_, query)| query.to_string());

        let body = "<!DOCTYPE html><html><body><p>You can close this window.</p></body></html>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;

        if let Some(query) = query {
            let pairs = url::form_urlencoded::parse(query.as_bytes())
                .map(|(k, v)| (k.into_owned(), v.into_owned()));
            if let Err(err) = complete_from_query(pairs).await {
                tracing::error!("Failed to handle OAuth callback: {err}");
            }
        }
    }
}

async fn complete_from_query(pairs: impl Iterator<Item = (String, String)>) -> Result<()> {
    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_description = None;
    for (key, value) in pairs {
        match key.as_str() {
            "code" => code = Some(value),
            "state" => state = Some(value),
            "error" => error = Some(value),
            "error_description" => error_description = Some(value),
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
        tracing::warn!("Redirect missing code/state parameters");
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
