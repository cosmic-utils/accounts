use std::collections::HashMap;
use std::str::FromStr;

use crate::{
    models::{Account, DbusProviderInfo, Provider, Service},
    proxy::{
        AccountAddedStream, AccountProxy, AccountRemovedStream, CredentialsProxy, ManagerProxy,
        ProviderProxy, RequestProxy,
    },
};
use uuid::Uuid;
use zbus::{Connection, fdo::Result, zvariant::OwnedObjectPath};

#[derive(Debug, Clone)]
pub struct AccountsClient {
    connection: Connection,
    manager: ManagerProxy<'static>,
}

impl AccountsClient {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;
        let manager = ManagerProxy::new(&connection).await?;
        Ok(Self {
            connection,
            manager,
        })
    }

    fn account_path(id: &Uuid) -> OwnedObjectPath {
        OwnedObjectPath::try_from(format!(
            "/dev/edfloreshz/Accounts/Accounts/{}",
            id.to_string().replace('-', "_")
        ))
        .expect("account object path is always a valid path")
    }

    async fn account_proxy(&self, path: OwnedObjectPath) -> Result<AccountProxy<'static>> {
        let builder = AccountProxy::builder(&self.connection)
            .path(path)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        builder
            .build()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    pub async fn request_proxy(&self, path: OwnedObjectPath) -> Result<RequestProxy<'static>> {
        let builder = RequestProxy::builder(&self.connection)
            .path(path)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        builder
            .build()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn account_from_proxy(proxy: &AccountProxy<'static>) -> Result<Account> {
        let id = Uuid::from_str(&proxy.id().await?)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let provider = proxy.provider_id().await?;
        let display_name = proxy.display_name().await?;
        let identity = proxy.identity().await?;
        let enabled = proxy.enabled().await?;
        let available_services = proxy.available_services().await?;
        let enabled_services = proxy.enabled_services().await?;
        let created_at = proxy.created_at().await.unwrap_or_default();
        let last_used = proxy.last_used().await.unwrap_or_default();
        let email = proxy.email().await.unwrap_or_default();

        let services = available_services
            .into_iter()
            .filter_map(Service::from_str)
            .map(|service| {
                let enabled = enabled_services.contains(&service.to_string());
                (service, enabled)
            })
            .collect();

        Ok(Account {
            id,
            provider,
            display_name,
            username: identity,
            email: (!email.is_empty()).then_some(email),
            enabled,
            created_at: chrono::DateTime::from_str(&created_at)
                .unwrap_or_else(|_| chrono::Utc::now()),
            last_used: chrono::DateTime::from_str(&last_used).ok(),
            services,
        })
    }
}

impl AccountsClient {
    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        let paths = self.manager.list_accounts().await?;
        let mut accounts = Vec::with_capacity(paths.len());
        for path in paths {
            let proxy = self.account_proxy(path).await?;
            accounts.push(Self::account_from_proxy(&proxy).await?);
        }
        Ok(accounts)
    }

    pub async fn list_enabled_accounts(&self, service: Service) -> Result<Vec<Account>> {
        let accounts = self.list_accounts().await?;
        Ok(accounts
            .into_iter()
            .filter(|a| a.enabled && matches!(a.services.get(&service), Some(true)))
            .collect())
    }

    /// Kicks off sign-in for `provider` and returns a proxy to the `Request` object that
    /// tracks it; the caller watches `StatusChanged` (or polls `Status`) to know when to
    /// open `InteractionUri` and when the flow reaches a terminal state.
    pub async fn create_account(&mut self, provider: &Provider) -> Result<RequestProxy<'static>> {
        let path = self
            .manager
            .create_account(provider, HashMap::new())
            .await?;
        self.request_proxy(path).await
    }

    pub async fn list_providers(&self) -> Result<Vec<DbusProviderInfo>> {
        let paths = self.manager.list_providers().await?;
        let mut providers = Vec::with_capacity(paths.len());
        for path in paths {
            let proxy = ProviderProxy::builder(&self.connection)
                .path(path)
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
                .build()
                .await
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
            providers.push(DbusProviderInfo {
                id: proxy.id().await?,
                name: proxy.name().await?,
                services: proxy.services().await?,
                icon: {
                    let icon = proxy.icon_name().await?;
                    (!icon.is_empty()).then_some(icon)
                },
            });
        }
        Ok(providers)
    }

    pub async fn get_account(&self, id: &Uuid) -> Result<Account> {
        let proxy = self.account_proxy(Self::account_path(id)).await?;
        Self::account_from_proxy(&proxy).await
    }

    pub async fn remove_account(&mut self, id: &Uuid) -> Result<()> {
        let proxy = self.account_proxy(Self::account_path(id)).await?;
        proxy.remove().await
    }

    pub async fn set_account_enabled(&mut self, id: &Uuid, enabled: bool) -> Result<()> {
        let proxy = self.account_proxy(Self::account_path(id)).await?;
        proxy.set_enabled(enabled).await
    }

    pub async fn set_service_enabled(
        &mut self,
        id: &Uuid,
        service: &Service,
        enabled: bool,
    ) -> Result<()> {
        let proxy = self.account_proxy(Self::account_path(id)).await?;
        if enabled {
            proxy.enable_service(&service.to_string()).await
        } else {
            proxy.disable_service(&service.to_string()).await
        }
    }

    pub async fn ensure_credentials(&mut self, id: &Uuid) -> Result<()> {
        let proxy = self.account_proxy(Self::account_path(id)).await?;
        proxy.ensure_credentials().await.map(|_| ())
    }

    /// Standing consent grants for this account: `(service, caller_identity, decision)`.
    pub async fn list_grants(&self, id: &Uuid) -> Result<Vec<(String, String, String)>> {
        let proxy = self.account_proxy(Self::account_path(id)).await?;
        proxy.list_grants().await
    }

    pub async fn revoke_grant(
        &mut self,
        id: &Uuid,
        service: &str,
        caller_identity: &str,
    ) -> Result<()> {
        let proxy = self.account_proxy(Self::account_path(id)).await?;
        proxy.revoke_grant(service, caller_identity).await
    }

    /// `(access_token, expires_at)` for one service, subject to polkit and the
    /// per-(account, service, caller) consent grant. Served by the `Credentials`
    /// interface on the account's own object path.
    pub async fn get_access_token(
        &mut self,
        id: &Uuid,
        service: &Service,
    ) -> Result<(String, i64)> {
        let proxy = CredentialsProxy::builder(&self.connection)
            .path(Self::account_path(id))
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
            .build()
            .await?;
        proxy.get_access_token(&service.to_string()).await
    }

    /// Re-runs the OAuth2 flow for an existing account and refreshes its stored
    /// credentials on success. Same `Request`-tracking pattern as `create_account`.
    pub async fn reauthenticate(&mut self, id: &Uuid) -> Result<RequestProxy<'static>> {
        let proxy = self.account_proxy(Self::account_path(id)).await?;
        let path = proxy.reauthenticate().await?;
        self.request_proxy(path).await
    }

    pub async fn receive_account_added(&self) -> zbus::Result<AccountAddedStream> {
        self.manager.receive_account_added().await
    }

    pub async fn receive_account_removed(&self) -> zbus::Result<AccountRemovedStream> {
        self.manager.receive_account_removed().await
    }
}
