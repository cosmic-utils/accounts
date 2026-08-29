use std::collections::HashMap;

use zbus::fdo::Result;
use zbus::proxy;
use zbus::zvariant::OwnedObjectPath;

#[proxy(
    default_service = "dev.edfloreshz.Accounts",
    default_path = "/dev/edfloreshz/Accounts/Manager",
    interface = "dev.edfloreshz.Accounts.Manager"
)]
pub trait Manager {
    async fn list_accounts(&self) -> Result<Vec<OwnedObjectPath>>;
    async fn list_providers(&self) -> Result<Vec<OwnedObjectPath>>;
    async fn create_account(
        &self,
        provider_id: &str,
        params: HashMap<String, String>,
    ) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn version(&self) -> Result<String>;

    #[zbus(signal)]
    fn account_added(account: OwnedObjectPath) -> Result<()>;

    #[zbus(signal)]
    fn account_removed(account: OwnedObjectPath) -> Result<()>;
}

#[proxy(
    default_service = "dev.edfloreshz.Accounts",
    interface = "dev.edfloreshz.Accounts.Request"
)]
pub trait Request {
    #[zbus(property)]
    fn status(&self) -> Result<String>;

    #[zbus(property)]
    fn interaction_uri(&self) -> Result<String>;

    #[zbus(property)]
    fn error_message(&self) -> Result<String>;

    #[zbus(property)]
    fn account(&self) -> Result<OwnedObjectPath>;

    async fn cancel(&self) -> Result<()>;

    #[zbus(signal, name = "StatusChanged")]
    fn request_status_changed(status: &str) -> Result<()>;
}

#[proxy(
    default_service = "dev.edfloreshz.Accounts",
    interface = "dev.edfloreshz.Accounts.Account"
)]
pub trait Account {
    #[zbus(property)]
    fn id(&self) -> Result<String>;

    #[zbus(property)]
    fn provider_id(&self) -> Result<String>;

    #[zbus(property)]
    fn display_name(&self) -> Result<String>;

    #[zbus(property)]
    fn set_display_name(&self, value: &str) -> Result<()>;

    #[zbus(property)]
    fn identity(&self) -> Result<String>;

    #[zbus(property)]
    fn enabled(&self) -> Result<bool>;

    #[zbus(property)]
    fn set_enabled(&self, value: bool) -> Result<()>;

    #[zbus(property)]
    fn available_services(&self) -> Result<Vec<String>>;

    #[zbus(property)]
    fn enabled_services(&self) -> Result<Vec<String>>;

    /// Extra property beyond the staged spec, kept so the UI can still show when an
    /// account was created/last used without a dedicated history store.
    #[zbus(property)]
    fn created_at(&self) -> Result<String>;

    #[zbus(property)]
    fn last_used(&self) -> Result<String>;

    #[zbus(property)]
    fn email(&self) -> Result<String>;

    async fn enable_service(&self, service: &str) -> Result<()>;
    async fn disable_service(&self, service: &str) -> Result<()>;
    async fn remove(&self) -> Result<()>;
    async fn ensure_credentials(&self) -> Result<()>;
    async fn reauthenticate(&self) -> Result<OwnedObjectPath>;

    /// `a(sss)` of `(service, caller_identity, decision)`.
    async fn list_grants(&self) -> Result<Vec<(String, String, String)>>;
    async fn revoke_grant(&self, service: &str, caller_identity: &str) -> Result<()>;

    #[zbus(signal)]
    fn services_changed(enabled_services: Vec<String>) -> Result<()>;

    /// Fired when the `Enabled` master switch is flipped through this interface,
    /// carrying the new value (alongside the standard `PropertiesChanged`).
    #[zbus(signal)]
    fn account_toggled(enabled: bool) -> Result<()>;
}

/// `dev.edfloreshz.Accounts.Credentials` — served on the same object path as the
/// owning `Account`. The only interface over which access tokens cross the bus.
#[proxy(
    default_service = "dev.edfloreshz.Accounts",
    interface = "dev.edfloreshz.Accounts.Credentials"
)]
pub trait Credentials {
    #[zbus(property)]
    fn auth_method(&self) -> Result<String>;

    /// Returns a valid access token and its expiry (unix seconds, 0 = n/a) for
    /// the named service, subject to polkit + the per-(account, service, caller)
    /// consent grant.
    async fn get_access_token(&self, service: &str) -> Result<(String, i64)>;

    async fn invalidate_token(&self) -> Result<()>;
}

#[proxy(
    default_service = "dev.edfloreshz.Accounts",
    interface = "dev.edfloreshz.Accounts.Provider"
)]
pub trait Provider {
    #[zbus(property)]
    fn id(&self) -> Result<String>;

    #[zbus(property)]
    fn name(&self) -> Result<String>;

    #[zbus(property)]
    fn icon_name(&self) -> Result<String>;

    #[zbus(property)]
    fn services(&self) -> Result<Vec<String>>;

    #[zbus(property)]
    fn auth_method(&self) -> Result<String>;
}

// The `Endpoint.*` interfaces say WHERE a service's data lives and HOW to
// authenticate to it — never the data itself. They are served on the owning
// Account's object path and appear/disappear as services are enabled/disabled.

#[proxy(
    interface = "dev.edfloreshz.Accounts.Endpoint.Calendar",
    default_service = "dev.edfloreshz.Accounts"
)]
pub trait Calendar {
    /// CalDAV collection/principal URL.
    #[zbus(property)]
    fn uri(&self) -> Result<String>;
    #[zbus(property)]
    fn auth_method(&self) -> Result<String>;
}

#[proxy(
    interface = "dev.edfloreshz.Accounts.Endpoint.Tasks",
    default_service = "dev.edfloreshz.Accounts"
)]
pub trait Tasks {
    /// CalDAV collection URL for VTODO components; may equal the calendar URL.
    #[zbus(property)]
    fn uri(&self) -> Result<String>;
    #[zbus(property)]
    fn auth_method(&self) -> Result<String>;
}

#[proxy(
    interface = "dev.edfloreshz.Accounts.Endpoint.Contacts",
    default_service = "dev.edfloreshz.Accounts"
)]
pub trait Contacts {
    /// CardDAV collection/principal URL.
    #[zbus(property)]
    fn uri(&self) -> Result<String>;
    #[zbus(property)]
    fn auth_method(&self) -> Result<String>;
}

#[proxy(
    interface = "dev.edfloreshz.Accounts.Endpoint.Mail",
    default_service = "dev.edfloreshz.Accounts"
)]
pub trait Mail {
    #[zbus(property)]
    fn imap_host(&self) -> Result<String>;
    #[zbus(property)]
    fn imap_port(&self) -> Result<u16>;
    #[zbus(property)]
    fn smtp_host(&self) -> Result<String>;
    #[zbus(property)]
    fn smtp_port(&self) -> Result<u16>;
    #[zbus(property)]
    fn auth_method(&self) -> Result<String>;
}

/// Implemented by a third-party service, not by the accounts daemon. Referenced
/// from a `.provider` manifest's `[handler]` section for providers whose auth
/// flow isn't a stock OAuth2 one the daemon already knows how to drive. This is
/// the entire extension point for non-standard auth — deliberately one method
/// wide; never add a second.
/// Reserved `params` key the daemon uses to pass an existing credential blob
/// (base64) back into `Authenticate` when refreshing a handler-based account —
/// the handler's "own later invocation" per the spec, keeping the interface one
/// method wide.
pub const HANDLER_BLOB_PARAM: &str = "dev.edfloreshz.Accounts.credential_blob";

#[proxy(interface = "dev.edfloreshz.Accounts.ProviderHandler")]
pub trait ProviderHandler {
    /// Drives whatever custom flow is needed and returns an opaque credential
    /// blob the daemon stores and later uses to mint access tokens. `params` is
    /// whatever `Manager.CreateAccount` was given, passed through. (`a{sv}` in
    /// the spec; kept as `a{ss}` here to match this codebase's existing param
    /// simplification.)
    async fn authenticate(&self, params: HashMap<String, String>) -> Result<(String, Vec<u8>)>;
}
