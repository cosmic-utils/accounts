use std::str::FromStr;

use crate::{
    models::{Account, Provider, Service},
    proxy::{
        AccountAddedStream, AccountChangedStream, AccountExistsStream, AccountRemovedStream,
        AccountsProxy,
    },
};
use uuid::Uuid;
use zbus::{Connection, fdo::Result};

#[derive(Debug, Clone)]
pub struct AccountsClient {
    proxy: AccountsProxy<'static>,
}

impl AccountsClient {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;
        let proxy = AccountsProxy::new(&connection).await?;
        Ok(Self { proxy })
    }
}

impl AccountsClient {
    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        self.proxy
            .list_accounts()
            .await
            .map(|accounts| accounts.into_iter().map(Into::into).collect())
    }

    pub async fn list_enabled_accounts(&self, service: Service) -> Result<Vec<Account>> {
        self.proxy.list_accounts().await.map(|accounts| {
            accounts
                .into_iter()
                .filter(|a| a.enabled && matches!(a.services.get(&service.to_string()), Some(true)))
                .map(Into::into)
                .collect()
        })
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
        let id = id.to_string();
        self.proxy.set_account_enabled(&id, enabled).await?;
        self.proxy.emit_account_changed(&id).await
    }

    pub async fn set_service_enabled(
        &mut self,
        id: &Uuid,
        service: &Service,
        enabled: bool,
    ) -> Result<()> {
        let id = id.to_string();
        self.proxy
            .set_service_enabled(&id, &service.to_string(), enabled)
            .await?;
        self.proxy.emit_account_changed(&id).await
    }

    pub async fn ensure_credentials(&mut self, id: &Uuid) -> Result<()> {
        self.proxy.ensure_credentials(&id.to_string()).await
    }

    pub async fn get_access_token(&mut self, id: &Uuid) -> Result<String> {
        let id = id.to_string();
        let access_token = self.proxy.get_access_token(&id).await?;
        Ok(access_token)
    }

    pub async fn get_refresh_token(&mut self, id: &Uuid) -> Result<String> {
        let id = id.to_string();
        let refresh_token = self.proxy.get_refresh_token(&id).await?;
        Ok(refresh_token)
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
