pub mod account;
pub mod auth;
pub mod error;
pub mod manager;
pub mod provider;
pub mod services;
pub mod storage;

use accounts_core::{ProviderRegistry, config::AccountsConfig, proxy::ManagerProxy};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, OnceCell};
use tracing::info;

use account::AccountInterface;
pub use error::{Error, Result};
use manager::ManagerInterface;
use provider::ProviderInterface;
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

    let config = Arc::new(Mutex::new(AccountsConfig::config()));
    let auth_manager = Arc::new(Mutex::new(
        auth::AuthManager::new()
            .await
            .map_err(|e| zbus::Error::Failure(e.to_string()))?,
    ));

    let accounts = config.lock().await.accounts.clone();

    let manager_iface = ManagerInterface::new(config.clone(), auth_manager.clone());

    let mut builder = zbus::connection::Builder::session()?
        .name("dev.edfloreshz.Accounts")?
        .serve_at("/dev/edfloreshz/Accounts/Manager", manager_iface)?;

    for account in &accounts {
        let path = manager::account_object_path(&account.id);
        builder = builder.serve_at(
            path,
            AccountInterface::new(account.id, config.clone(), auth_manager.clone()),
        )?;
    }

    if let Some(registry) = REGISTRY.get() {
        for manifest in registry.list() {
            let path = format!(
                "/dev/edfloreshz/Accounts/Providers/{}",
                manifest.provider.id
            );
            builder = builder.serve_at(path, ProviderInterface::new(manifest.clone()))?;
        }
    }

    CONNECTION.set(builder.build().await?).unwrap();

    for account in &accounts {
        let services = ServiceFactory::create_services(account);
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

    let Ok(connection) = Connection::session().await else {
        tracing::error!("Accounts for COSMIC client failed to initialize");
        return Ok(());
    };
    let Ok(mut manager) = ManagerProxy::new(&connection).await else {
        tracing::error!("Failed to reach the Manager object");
        return Ok(());
    };

    if let Some(error) = error {
        let reason = error_description.unwrap_or(error);
        tracing::error!("Authentication failed: {reason}");
        report_authentication_failure(&manager, &reason).await;
        return Ok(());
    }

    let (Some(authorization_code), Some(csrf_token)) = (code, state) else {
        tracing::warn!("Redirect missing code/state parameters");
        report_authentication_failure(&manager, "The provider did not return a sign-in code")
            .await;
        return Ok(());
    };

    match manager
        .complete_authentication(&csrf_token, &authorization_code)
        .await
    {
        Ok(account_path) => {
            tracing::info!("User authenticated, account object at {}", *account_path);
        }
        Err(err) => {
            tracing::error!("Failed to authenticate user: {}", err);
            report_authentication_failure(&manager, &err.to_string()).await;
        }
    }

    Ok(())
}

/// Lets the front end know that a sign-in attempt it is waiting on will never complete.
async fn report_authentication_failure(
    manager: &accounts_core::proxy::ManagerProxy<'_>,
    reason: &str,
) {
    if let Err(err) = manager.emit_authentication_failed(reason).await {
        tracing::error!("Failed to signal the authentication failure: {}", err);
    }
}
