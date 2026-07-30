use std::collections::HashMap;

use zbus::fdo::Result;
use zbus::proxy;

use crate::models::{DbusAccount, DbusProviderInfo};

#[proxy(
    default_service = "dev.edfloreshz.Accounts",
    default_path = "/dev/edfloreshz/Accounts/Account",
    interface = "dev.edfloreshz.Accounts.Account"
)]
pub trait Accounts {
    async fn list_accounts(&self) -> Result<Vec<DbusAccount>>;
    async fn get_account(&self, id: &str) -> Result<DbusAccount>;
    /// Every known provider and the services it advertises support for, sourced
    /// from the manifest registry. Does not require any provider process to be running.
    async fn list_providers(&self) -> Result<Vec<DbusProviderInfo>>;
    async fn start_authentication(&mut self, provider_name: &str) -> Result<String>;
    async fn complete_authentication(
        &mut self,
        csrf_token: &str,
        authorization_code: &str,
    ) -> Result<String>;
    async fn remove_account(&mut self, id: &str) -> Result<()>;
    async fn set_account_enabled(&mut self, id: &str, enabled: bool) -> Result<()>;
    async fn set_service_enabled(&mut self, id: &str, service: &str, enabled: bool) -> Result<()>;
    async fn get_access_token(&mut self, id: &str) -> Result<String>;
    async fn get_refresh_token(&mut self, id: &str) -> Result<String>;
    async fn ensure_credentials(&mut self, id: &str) -> Result<()>;

    async fn emit_account_added(&self, account_id: &str) -> Result<()>;
    async fn emit_account_removed(&self, account_id: &str) -> Result<()>;
    async fn emit_account_changed(&self, account_id: &str) -> Result<()>;
    async fn emit_account_exists(&self) -> Result<()>;

    #[zbus(signal)]
    fn account_added(account_id: &str) -> Result<()>;

    #[zbus(signal)]
    fn account_removed(account_id: &str) -> Result<()>;

    #[zbus(signal)]
    fn account_changed(account_id: &str) -> Result<()>;

    #[zbus(signal)]
    fn account_exists() -> Result<()>;
}

#[proxy(
    interface = "dev.edfloreshz.Accounts.Calendar",
    default_service = "dev.edfloreshz.Accounts"
)]
pub trait Calendar {
    async fn uri(&self) -> Result<String>;
    async fn accept_ssl_errors(&self) -> Result<bool>;
}

#[proxy(
    interface = "dev.edfloreshz.Accounts.Todo",
    default_service = "dev.edfloreshz.Accounts"
)]
pub trait Todo {
    async fn uri(&self) -> Result<String>;
    async fn accept_ssl_errors(&self) -> Result<bool>;
}

#[proxy(
    interface = "dev.edfloreshz.Accounts.Contacts",
    default_service = "dev.edfloreshz.Accounts"
)]
pub trait Contacts {
    async fn uri(&self) -> Result<String>;
    async fn accept_ssl_errors(&self) -> Result<bool>;
}

#[proxy(
    interface = "dev.edfloreshz.Accounts.Mail",
    default_service = "dev.edfloreshz.Accounts"
)]
pub trait Mail {
    async fn email_address(&self) -> Result<String>;
    async fn name(&self) -> Result<String>;

    async fn imap_host(&self) -> Result<String>;
    async fn imap_user_name(&self) -> Result<String>;
    async fn imap_supported(&self) -> Result<bool>;
    async fn imap_use_ssl(&self) -> Result<bool>;
    async fn imap_use_tls(&self) -> Result<bool>;
    async fn imap_accept_ssl_errors(&self) -> Result<bool>;

    async fn smtp_host(&self) -> Result<String>;
    async fn smtp_user_name(&self) -> Result<String>;
    async fn smtp_supported(&self) -> Result<bool>;
    async fn smtp_use_auth(&self) -> Result<bool>;
    async fn smtp_use_ssl(&self) -> Result<bool>;
    async fn smtp_use_tls(&self) -> Result<bool>;
    async fn smtp_accept_ssl_errors(&self) -> Result<bool>;
    async fn smtp_auth_login(&self) -> Result<bool>;
    async fn smtp_auth_plain(&self) -> Result<bool>;
    async fn smtp_auth_xoauth2(&self) -> Result<bool>;
}

/// Implemented by each provider process (first-party or third-party). Deliberately
/// small: token exchange/refresh and secret storage stay in the daemon, so a provider
/// only ever receives a short-lived access token for the one call being made.
/// `default_service` is intentionally omitted since it differs per provider (taken
/// from the provider's manifest `dbus_name`) — construct with
/// `Provider1Proxy::new(&connection, dbus_name)`.
#[proxy(
    default_path = "/dev/edfloreshz/Accounts/Provider",
    interface = "dev.edfloreshz.Accounts.Provider1"
)]
pub trait Provider1 {
    /// Given a fresh access token, return normalized user info. Keys: `display_name`,
    /// `username`, and optionally `email`.
    async fn get_user_info(&self, access_token: &str) -> Result<HashMap<String, String>>;

    /// Static per-service connection info (e.g. a CalDAV URI for `"calendar"`).
    /// Doesn't need a token: this is provider/service configuration, not per-account data.
    async fn get_service_config(&self, service: &str) -> Result<HashMap<String, String>>;
}
