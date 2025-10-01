use zbus::fdo::Result;
use zbus::proxy;

use crate::models::DbusAccount;

#[proxy(
    default_service = "dev.edfloreshz.Accounts",
    default_path = "/dev/edfloreshz/Accounts/Account",
    interface = "dev.edfloreshz.Accounts.Account"
)]
pub trait Accounts {
    async fn list_accounts(&self) -> Result<Vec<DbusAccount>>;
    async fn get_account(&self, id: &str) -> Result<DbusAccount>;
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
    interface = "dev.edfloreshz.Accounts",
    default_service = "dev.edfloreshz.Accounts.Calendar"
)]
pub trait Calendar {
    async fn uri(&self) -> Result<String>;
    async fn accept_ssl_errors(&self) -> Result<bool>;
}
