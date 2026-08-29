use crate::daemon::{
    Error,
    auth::AuthManager,
    request::{RequestInterface, RequestState, RequestStatus, request_object_path},
};
use accounts_core::config::AccountsConfig;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use uuid::Uuid;
use zbus::{
    Connection,
    fdo::Result,
    interface,
    object_server::SignalEmitter,
    zvariant::{ObjectPath, OwnedObjectPath},
};

/// A `Request` object is left registered for this long after reaching a terminal state,
/// so a caller reading `Status` right after `StatusChanged` fires doesn't race removal.
const REQUEST_GRACE_PERIOD: Duration = Duration::from_secs(5);
/// OAuth2 interactive flows can be slow; this is unrelated to any later consent-prompt timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub fn account_object_path(id: &uuid::Uuid) -> OwnedObjectPath {
    OwnedObjectPath::try_from(format!(
        "/dev/edfloreshz/Accounts/Accounts/{}",
        id.to_string().replace('-', "_")
    ))
    .expect("account object path is always a valid path")
}

pub struct ManagerInterface {
    pub(crate) config: Arc<Mutex<AccountsConfig>>,
    pub(crate) auth_manager: Arc<Mutex<AuthManager>>,
}

impl ManagerInterface {
    pub fn new(config: Arc<Mutex<AccountsConfig>>, auth_manager: Arc<Mutex<AuthManager>>) -> Self {
        Self {
            config,
            auth_manager,
        }
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Manager")]
impl ManagerInterface {
    async fn list_accounts(&self) -> Result<Vec<OwnedObjectPath>> {
        let config = self.config.lock().await;
        Ok(config
            .accounts
            .iter()
            .map(|account| account_object_path(&account.id))
            .collect())
    }

    async fn list_providers(&self) -> Result<Vec<OwnedObjectPath>> {
        let Some(registry) = crate::daemon::REGISTRY.get() else {
            return Ok(Vec::new());
        };
        Ok(registry
            .list()
            .into_iter()
            .filter_map(|manifest| {
                ObjectPath::try_from(format!(
                    "/dev/edfloreshz/Accounts/Providers/{}",
                    manifest.provider.id
                ))
                .ok()
                .map(|p| p.into())
            })
            .collect())
    }

    #[zbus(property)]
    async fn version(&self) -> Result<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn create_account(
        &self,
        provider_id: &str,
        _params: HashMap<String, String>,
    ) -> Result<OwnedObjectPath> {
        let Some(registry) = crate::daemon::REGISTRY.get() else {
            return Err(Error::InvalidProviderConfig.into());
        };
        if registry.get(provider_id).is_none() {
            return Err(Error::InvalidProvider(provider_id.to_string()).into());
        }

        let path = create_request(provider_id.to_string(), None, self.auth_manager.clone()).await?;

        Ok(path)
    }

    #[zbus(signal)]
    pub(crate) async fn account_added(
        emitter: &SignalEmitter<'_>,
        account: OwnedObjectPath,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub(crate) async fn account_removed(
        emitter: &SignalEmitter<'_>,
        account: OwnedObjectPath,
    ) -> zbus::Result<()>;
}

/// Registers a new `Request` object at `Status = "pending"` and spawns the async task that
/// drives the OAuth2 flow. Shared by `Manager.CreateAccount` and `Account.Reauthenticate`;
/// `existing_account` distinguishes creating a brand-new account from refreshing one's
/// credentials in place.
pub(crate) async fn create_request(
    provider_id: String,
    existing_account: Option<Uuid>,
    auth_manager: Arc<Mutex<AuthManager>>,
) -> Result<OwnedObjectPath> {
    let Some(connection) = crate::daemon::CONNECTION.get() else {
        return Err(Error::InvalidProviderConfig.into());
    };
    let Some(requests) = crate::daemon::REQUESTS.get() else {
        return Err(Error::InvalidProviderConfig.into());
    };

    let request_id = Uuid::new_v4().to_string();
    let path = request_object_path(&request_id);
    let state = Arc::new(Mutex::new(RequestState::default()));

    requests
        .lock()
        .await
        .insert(request_id.clone(), state.clone());

    connection
        .object_server()
        .at(
            path.clone(),
            RequestInterface::new(request_id.clone(), state.clone(), auth_manager.clone()),
        )
        .await?;

    tokio::spawn(run_oauth_flow(
        provider_id,
        existing_account,
        request_id,
        state,
        auth_manager,
        connection.clone(),
    ));

    Ok(path)
}

async fn run_oauth_flow(
    provider_id: String,
    existing_account: Option<Uuid>,
    request_id: String,
    state: Arc<Mutex<RequestState>>,
    auth_manager: Arc<Mutex<AuthManager>>,
    connection: Connection,
) {
    let url_result = auth_manager
        .lock()
        .await
        .start_auth_flow(provider_id, request_id.clone(), existing_account)
        .await;

    let csrf_token = match url_result {
        Ok((url, csrf_token)) => {
            {
                let mut state = state.lock().await;
                state.status = RequestStatus::NeedsInteraction;
                state.interaction_uri = url;
                state.csrf_token = Some(csrf_token.clone());
            }
            emit_status_changed(&connection, &request_id, RequestStatus::NeedsInteraction).await;
            csrf_token
        }
        Err(err) => {
            tracing::error!("Failed to start authentication flow: {err}");
            fail_request(&connection, &request_id, &state, err.to_string()).await;
            return;
        }
    };

    tokio::spawn(request_timeout(
        request_id,
        csrf_token,
        state,
        auth_manager,
        connection,
    ));
}

async fn request_timeout(
    request_id: String,
    csrf_token: String,
    state: Arc<Mutex<RequestState>>,
    auth_manager: Arc<Mutex<AuthManager>>,
    connection: Connection,
) {
    tokio::time::sleep(REQUEST_TIMEOUT).await;

    let already_terminal = state.lock().await.status.is_terminal();
    if already_terminal {
        return;
    }

    auth_manager.lock().await.discard_pending(&csrf_token);
    fail_request(
        &connection,
        &request_id,
        &state,
        "The sign-in attempt timed out".to_string(),
    )
    .await;
}

pub(crate) async fn fail_request(
    connection: &Connection,
    request_id: &str,
    state: &Arc<Mutex<RequestState>>,
    reason: String,
) {
    {
        let mut state = state.lock().await;
        if state.status.is_terminal() {
            return;
        }
        state.status = RequestStatus::Failed;
        state.error_message = reason;
    }
    emit_status_changed(connection, request_id, RequestStatus::Failed).await;
    schedule_cleanup(connection.clone(), request_id.to_string());
}

pub(crate) async fn succeed_request(
    connection: &Connection,
    request_id: &str,
    state: &Arc<Mutex<RequestState>>,
    account_path: OwnedObjectPath,
) {
    {
        let mut state = state.lock().await;
        state.status = RequestStatus::Succeeded;
        state.account = account_path;
    }
    emit_status_changed(connection, request_id, RequestStatus::Succeeded).await;
    schedule_cleanup(connection.clone(), request_id.to_string());
}

async fn emit_status_changed(connection: &Connection, request_id: &str, status: RequestStatus) {
    let path = request_object_path(request_id);
    if let Ok(iface_ref) = connection
        .object_server()
        .interface::<_, RequestInterface>(path)
        .await
    {
        let _ = RequestInterface::request_status_changed(
            iface_ref.signal_emitter(),
            status.as_str().to_string(),
        )
        .await;
    }
}

fn schedule_cleanup(connection: Connection, request_id: String) {
    tokio::spawn(async move {
        tokio::time::sleep(REQUEST_GRACE_PERIOD).await;
        let path = request_object_path(&request_id);
        let _ = connection
            .object_server()
            .remove::<RequestInterface, _>(path)
            .await;
        if let Some(requests) = crate::daemon::REQUESTS.get() {
            requests.lock().await.remove(&request_id);
        }
    });
}
