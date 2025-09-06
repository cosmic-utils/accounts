use zbus::Connection;

use crate::Account;

pub struct CosmicAccountsClient {
    proxy: zbus::Proxy<'static>,
}

impl CosmicAccountsClient {
    pub async fn new() -> Result<Self, zbus::Error> {
        let connection = Connection::session().await?;
        let proxy = zbus::Proxy::new(
            &connection,
            "com.system76.CosmicAccounts",
            "/com/system76/CosmicAccounts",
            "com.system76.CosmicAccounts",
        )
        .await?;
        Ok(Self { proxy })
    }

    pub async fn list_accounts(&self) -> Result<Vec<Account>, zbus::Error> {
        let accounts: Vec<String> = self.proxy.call("ListAccounts", &()).await?;
        let accounts = accounts
            .iter()
            .map(|account| serde_json::from_str(account).unwrap())
            .collect();
        Ok(accounts)
    }

    pub async fn get_account(&self, id: &str) -> Result<String, zbus::Error> {
        let account = self.proxy.call("GetAccount", &(id)).await?;
        Ok(account)
    }

    pub async fn remove_account(&self, id: &str) -> Result<(), zbus::Error> {
        self.proxy.call::<_, _, ()>("RemoveAccount", &(id)).await?;
        Ok(())
    }

    pub async fn set_account_enabled(&self, id: &str, enabled: bool) -> Result<(), zbus::Error> {
        self.proxy
            .call::<_, _, ()>("SetAccountEnabled", &(id, enabled))
            .await?;
        Ok(())
    }

    pub async fn get_access_token(&self, id: &str) -> Result<String, zbus::Error> {
        let token = self.proxy.call("GetAccessToken", &(id)).await?;
        Ok(token)
    }

    pub async fn start_authentication(&self, id: &str) -> Result<(), zbus::Error> {
        self.proxy
            .call::<_, _, ()>("StartAuthentication", &(id))
            .await?;
        Ok(())
    }

    pub async fn complete_authentication(&self, id: &str, code: &str) -> Result<(), zbus::Error> {
        self.proxy
            .call::<_, _, ()>("CompleteAuthentication", &(id, code))
            .await?;
        Ok(())
    }
}
