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
    async fn start_authentication(&mut self, provider_id: &str) -> Result<String>;
    async fn complete_authentication(
        &mut self,
        csrf_token: &str,
        authorization_code: &str,
    ) -> Result<OwnedObjectPath>;

    async fn emit_authentication_failed(&self, reason: &str) -> Result<()>;

    #[zbus(property)]
    fn version(&self) -> Result<String>;

    #[zbus(signal)]
    fn account_added(account: OwnedObjectPath) -> Result<()>;

    #[zbus(signal)]
    fn account_removed(account: OwnedObjectPath) -> Result<()>;

    #[zbus(signal)]
    fn authentication_failed(reason: &str) -> Result<()>;
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
    async fn get_access_token(&self) -> Result<String>;
    async fn ensure_credentials(&self) -> Result<()>;

    #[zbus(signal)]
    fn services_changed(enabled_services: Vec<String>) -> Result<()>;
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

#[proxy(
    default_path = "/dev/edfloreshz/Accounts/Provider",
    interface = "dev.edfloreshz.Accounts.Provider1"
)]
pub trait Provider1 {
    async fn get_user_info(&self, access_token: &str) -> Result<HashMap<String, String>>;

    async fn get_service_config(&self, service: &str) -> Result<HashMap<String, String>>;
}
