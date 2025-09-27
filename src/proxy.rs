use zbus::fdo::Result;
use zbus::proxy;

use crate::models::DbusAccount;

#[proxy(
    interface = "com.system76.CosmicAccounts",
    default_path = "/com/system76/CosmicAccounts",
    default_service = "com.system76.CosmicAccounts"
)]
trait CosmicAccounts {
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
    async fn get_access_token(&mut self, id: &str) -> Result<String>;
}
