use std::sync::Arc;

use tokio::sync::Mutex;
use zbus::{fdo::Result, interface, object_server::SignalEmitter, zvariant::OwnedObjectPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
    Pending,
    NeedsInteraction,
    Succeeded,
    Failed,
    Cancelled,
}

impl RequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RequestStatus::Pending => "pending",
            RequestStatus::NeedsInteraction => "needs-interaction",
            RequestStatus::Succeeded => "succeeded",
            RequestStatus::Failed => "failed",
            RequestStatus::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            RequestStatus::Succeeded | RequestStatus::Failed | RequestStatus::Cancelled
        )
    }
}

/// Object-path properties can't be null over D-Bus, so "unset" is represented as the
/// root path `/`, which is never a valid account/request path in this tree.
pub fn empty_object_path() -> OwnedObjectPath {
    OwnedObjectPath::try_from("/").expect("root path is always valid")
}

#[derive(Debug)]
pub struct RequestState {
    pub status: RequestStatus,
    pub interaction_uri: String,
    pub error_message: String,
    pub account: OwnedObjectPath,
    /// CSRF token correlating this request with the pending OAuth2 flow in `AuthManager`,
    /// so `Cancel` and the timeout task can evict the matching `pending_auth` entry.
    pub csrf_token: Option<String>,
}

impl Default for RequestState {
    fn default() -> Self {
        Self {
            status: RequestStatus::Pending,
            interaction_uri: String::new(),
            error_message: String::new(),
            account: empty_object_path(),
            csrf_token: None,
        }
    }
}

pub type SharedRequestState = Arc<Mutex<RequestState>>;

pub fn request_object_path(id: &str) -> OwnedObjectPath {
    // D-Bus path elements are `[A-Za-z0-9_]` only; a UUID's dashes are not valid.
    OwnedObjectPath::try_from(format!(
        "/dev/edfloreshz/Accounts/Requests/{}",
        id.replace('-', "_")
    ))
    .expect("request object path is always a valid path")
}

pub struct RequestInterface {
    pub(crate) id: String,
    pub(crate) state: SharedRequestState,
    pub(crate) auth_manager: Arc<Mutex<super::auth::AuthManager>>,
}

impl RequestInterface {
    pub fn new(
        id: String,
        state: SharedRequestState,
        auth_manager: Arc<Mutex<super::auth::AuthManager>>,
    ) -> Self {
        Self {
            id,
            state,
            auth_manager,
        }
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Request")]
impl RequestInterface {
    #[zbus(property)]
    async fn status(&self) -> Result<String> {
        Ok(self.state.lock().await.status.as_str().to_string())
    }

    #[zbus(property)]
    async fn interaction_uri(&self) -> Result<String> {
        Ok(self.state.lock().await.interaction_uri.clone())
    }

    #[zbus(property)]
    async fn error_message(&self) -> Result<String> {
        Ok(self.state.lock().await.error_message.clone())
    }

    #[zbus(property)]
    async fn account(&self) -> Result<OwnedObjectPath> {
        Ok(self.state.lock().await.account.clone())
    }

    async fn cancel(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> Result<()> {
        let csrf_token = {
            let mut state = self.state.lock().await;
            if state.status.is_terminal() {
                return Ok(());
            }
            state.status = RequestStatus::Cancelled;
            state.csrf_token.take()
        };

        if let Some(csrf_token) = csrf_token {
            self.auth_manager.lock().await.discard_pending(&csrf_token);
        }

        if let Some(requests) = crate::daemon::REQUESTS.get() {
            requests.lock().await.remove(&self.id);
        }

        Self::request_status_changed(&emitter, RequestStatus::Cancelled.as_str().to_string())
            .await
            .map_err(Into::into)
    }

    #[zbus(signal, name = "StatusChanged")]
    pub(crate) async fn request_status_changed(
        emitter: &SignalEmitter<'_>,
        status: String,
    ) -> zbus::Result<()>;
}
