use std::str::FromStr;

use crate::{
    models::{Account, Provider},
    proxy::{
        AccountAddedStream, AccountChangedStream, AccountExistsStream, AccountRemovedStream,
        CosmicAccountsProxy,
    },
};
use uuid::Uuid;
use zbus::{fdo::Result, Connection};

#[derive(Debug, Clone)]
pub struct CosmicAccountsClient {
    proxy: CosmicAccountsProxy<'static>,
}

impl<'a> CosmicAccountsClient {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;
        let proxy = CosmicAccountsProxy::new(&connection).await?;
        Ok(Self { proxy })
    }
}

impl CosmicAccountsClient {
    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        self.proxy
            .list_accounts()
            .await
            .map(|accounts| accounts.into_iter().map(Into::into).collect())
    }

    pub async fn start_authentication(&mut self, provider: &Provider) -> Result<String> {
        self.proxy.start_authentication(&provider.to_string()).await
    }

    pub async fn complete_authentication(
        &mut self,
        csrf_token: &str,
        authorization_code: &str,
    ) -> Result<Uuid> {
        let account_id = self
            .proxy
            .complete_authentication(csrf_token, authorization_code)
            .await?;
        Uuid::from_str(&account_id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    pub async fn get_account(&self, id: &str) -> Result<Account> {
        self.proxy.get_account(id).await.map(Into::into)
    }

    pub async fn remove_account(&mut self, id: &Uuid) -> Result<()> {
        self.proxy.remove_account(&id.to_string()).await
    }

    pub async fn set_account_enabled(&mut self, id: &Uuid, enabled: bool) -> Result<()> {
        self.proxy
            .set_account_enabled(&id.to_string(), enabled)
            .await
    }

    /// Signals
    pub async fn account_added(&self, account_id: &Uuid) -> Result<()> {
        self.proxy.emit_account_added(&account_id.to_string()).await
    }

    pub async fn account_removed(&self, account_id: &Uuid) -> Result<()> {
        self.proxy
            .emit_account_removed(&account_id.to_string())
            .await
    }

    pub async fn account_changed(&self, account_id: &Uuid) -> Result<()> {
        self.proxy
            .emit_account_changed(&account_id.to_string())
            .await
    }

    pub async fn account_exists(&self) -> Result<()> {
        self.proxy.emit_account_exists().await
    }

    pub async fn receive_account_added(&self) -> zbus::Result<AccountAddedStream> {
        self.proxy.receive_account_added().await
    }

    pub async fn receive_account_removed(&self) -> zbus::Result<AccountRemovedStream> {
        self.proxy.receive_account_removed().await
    }

    pub async fn receive_account_changed(&self) -> zbus::Result<AccountChangedStream> {
        self.proxy.receive_account_changed().await
    }

    pub async fn receive_account_exists(&self) -> zbus::Result<AccountExistsStream> {
        self.proxy.receive_account_exists().await
    }
}
