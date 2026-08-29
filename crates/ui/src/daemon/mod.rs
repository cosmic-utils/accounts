pub mod account;
pub mod auth;
pub mod caller;
pub mod consent;
pub mod credentials;
pub mod error;
pub mod grants;
pub mod manager;
pub mod polkit;
pub mod provider;
pub mod request;
pub mod services;
pub mod storage;

use accounts_core::{ProviderRegistry, config::AccountsConfig};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, OnceCell};
use tracing::info;
use uuid::Uuid;

use account::AccountInterface;
use credentials::CredentialsInterface;
pub use error::{Error, Result};
use grants::GrantStore;
use manager::ManagerInterface;
use provider::ProviderInterface;
use request::SharedRequestState;
use services::ServiceFactory;
use zbus::Connection;

/// Shared set of account ids whose cached token a caller reported dead via
/// `Credentials.InvalidateToken`; the next `GetAccessToken` forces a refresh.
type StaleTokens = Arc<Mutex<HashSet<Uuid>>>;

pub static CONNECTION: OnceCell<Connection> = OnceCell::const_new();
pub static REGISTRY: OnceCell<ProviderRegistry> = OnceCell::const_new();
/// Live `Request` objects keyed by id, so the OAuth callback handler can locate the one
/// that corresponds to a given CSRF token/`state` and drive it to a terminal status.
pub static REQUESTS: OnceCell<Arc<Mutex<HashMap<String, SharedRequestState>>>> =
    OnceCell::const_new();

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
    REQUESTS
        .set(Arc::new(Mutex::new(HashMap::new())))
        .expect("requests registry set once at startup");

    info!("Setting up D-Bus connection...");

    let config = Arc::new(Mutex::new(AccountsConfig::config()));
    let auth_manager = Arc::new(Mutex::new(
        auth::AuthManager::new()
            .await
            .map_err(|e| zbus::Error::Failure(e.to_string()))?,
    ));

    let accounts = config.lock().await.accounts.clone();

    let grants = GrantStore::open()
        .await
        .map_err(|e| zbus::Error::Failure(e.to_string()))?;
    let stale: StaleTokens = Arc::new(Mutex::new(HashSet::new()));

    let manager_iface = ManagerInterface::new(config.clone(), auth_manager.clone());

    let mut builder = zbus::connection::Builder::session()?
        .name("dev.edfloreshz.Accounts")?
        .serve_at("/dev/edfloreshz/Accounts/Manager", manager_iface)?;

    for account in &accounts {
        let path = manager::account_object_path(&account.id);
        builder = builder.serve_at(
            path.clone(),
            AccountInterface::new(
                account.id,
                config.clone(),
                auth_manager.clone(),
                grants.clone(),
            ),
        )?;
        builder = builder.serve_at(
            path,
            CredentialsInterface::new(
                account.id,
                config.clone(),
                auth_manager.clone(),
                grants.clone(),
                stale.clone(),
            ),
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
    tokio::spawn(run_callback_server(
        listener,
        config.clone(),
        auth_manager.clone(),
        grants.clone(),
        stale.clone(),
    ));

    info!("Accounts for COSMIC daemon started successfully");

    std::future::pending::<()>().await;
    Ok(())
}

async fn run_callback_server(
    listener: TcpListener,
    config: Arc<Mutex<AccountsConfig>>,
    auth_manager: Arc<Mutex<auth::AuthManager>>,
    grants: GrantStore,
    stale: StaleTokens,
) {
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
            complete_from_query(
                pairs,
                config.clone(),
                auth_manager.clone(),
                grants.clone(),
                stale.clone(),
            )
            .await;
        }
    }
}

/// Drives the `Request` object correlated with the redirect's `state` (CSRF token) to a
/// terminal status, in-process rather than routing back through a self D-Bus call: request
/// state already lives in daemon-local `Arc<Mutex<..>>`s that this handler has direct access to.
async fn complete_from_query(
    pairs: impl Iterator<Item = (String, String)>,
    config: Arc<Mutex<AccountsConfig>>,
    auth_manager: Arc<Mutex<auth::AuthManager>>,
    grants: GrantStore,
    stale: StaleTokens,
) {
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

    let Some(connection) = CONNECTION.get() else {
        tracing::error!("D-Bus connection not available for the OAuth callback");
        return;
    };
    let Some(requests) = REQUESTS.get() else {
        tracing::error!("Request registry not available for the OAuth callback");
        return;
    };

    if let Some(error) = error {
        let reason = error_description.unwrap_or(error);
        tracing::error!("Authentication failed: {reason}");
        if let Some(csrf_token) = &state {
            fail_by_csrf(connection, requests, &auth_manager, csrf_token, reason).await;
        }
        return;
    }

    let (Some(authorization_code), Some(csrf_token)) = (code, state) else {
        tracing::warn!("Redirect missing code/state parameters");
        return;
    };

    let completed = {
        let mut auth_manager = auth_manager.lock().await;
        auth_manager
            .complete_auth_flow(csrf_token, authorization_code)
            .await
    };

    let completed = match completed {
        Ok(completed) => completed,
        Err(err) => {
            tracing::error!("Failed to authenticate user: {err}");
            return;
        }
    };

    let auth::CompletedAuth {
        request_id,
        existing_account,
        account,
    } = completed;

    let Some(state) = ({
        let requests = requests.lock().await;
        requests.get(&request_id).cloned()
    }) else {
        tracing::error!("No pending Request found for id {request_id}");
        return;
    };

    if existing_account.is_some() {
        {
            let mut config = config.lock().await;
            if let Err(err) = config.save_account(&account) {
                tracing::error!("Failed to persist refreshed account: {err}");
            }
        }
        // Credentials are fresh again; drop any stale-token marker.
        stale.lock().await.remove(&account.id);
        let path = manager::account_object_path(&account.id);
        manager::succeed_request(connection, &request_id, &state, path).await;
        return;
    }

    {
        let mut config = config.lock().await;
        if let Err(err) = config.save_account(&account) {
            tracing::error!("Failed to save account: {err}");
            manager::fail_request(connection, &request_id, &state, err.to_string()).await;
            return;
        }
    }

    let path = manager::account_object_path(&account.id);

    if let Err(err) = connection
        .object_server()
        .at(
            path.clone(),
            AccountInterface::new(
                account.id,
                config.clone(),
                auth_manager.clone(),
                grants.clone(),
            ),
        )
        .await
    {
        tracing::error!("Failed to register account object: {err}");
    }

    if let Err(err) = connection
        .object_server()
        .at(
            path.clone(),
            CredentialsInterface::new(
                account.id,
                config.clone(),
                auth_manager.clone(),
                grants.clone(),
                stale.clone(),
            ),
        )
        .await
    {
        tracing::error!("Failed to register credentials object: {err}");
    }

    for service in ServiceFactory::create_services(&account) {
        if let Err(err) = service.add_service().await {
            tracing::error!("Failed to add service: {err}");
        }
    }

    if let Ok(iface_ref) = connection
        .object_server()
        .interface::<_, ManagerInterface>("/dev/edfloreshz/Accounts/Manager")
        .await
    {
        let _ = ManagerInterface::account_added(iface_ref.signal_emitter(), path.clone()).await;
    }

    manager::succeed_request(connection, &request_id, &state, path).await;
}

async fn fail_by_csrf(
    connection: &Connection,
    requests: &Arc<Mutex<HashMap<String, SharedRequestState>>>,
    auth_manager: &Arc<Mutex<auth::AuthManager>>,
    csrf_token: &str,
    reason: String,
) {
    let request_id = auth_manager.lock().await.request_id_for_csrf(csrf_token);
    let Some(request_id) = request_id else {
        return;
    };
    auth_manager.lock().await.discard_pending(csrf_token);

    let state = {
        let requests = requests.lock().await;
        requests.get(&request_id).cloned()
    };
    let Some(state) = state else {
        return;
    };

    manager::fail_request(connection, &request_id, &state, reason).await;
}
