use uuid::Uuid;
use zbus::Connection;

use crate::{proxy::CosmicAccountsProxy, Account, Provider, Result};

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
            .map_err(Into::into)
    }

    pub async fn start_authentication(&mut self, provider: &Provider) -> Result<String> {
        self.proxy
            .start_authentication(&provider.to_string())
            .await
            .map_err(Into::into)
    }

    pub async fn complete_authentication(
        &mut self,
        csrf_token: &str,
        authorization_code: &str,
    ) -> Result<String> {
        self.proxy
            .complete_authentication(csrf_token, authorization_code)
            .await
            .map_err(Into::into)
    }

    pub async fn get_account(&self, id: &str) -> Result<Account> {
        self.proxy
            .get_account(id)
            .await
            .map(Into::into)
            .map_err(Into::into)
    }

    pub async fn remove_account(&mut self, id: &Uuid) -> Result<()> {
        self.proxy
            .remove_account(&id.to_string())
            .await
            .map(Into::into)
            .map_err(Into::into)
    }
}
