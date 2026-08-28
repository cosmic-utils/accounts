use std::collections::HashMap;

use accounts_core::{
    AccountService, ServiceConfig,
    models::{Account, Service},
    proxy::Provider1Proxy,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use zbus::{
    Connection,
    fdo::{Error, Result},
    interface,
};

use crate::daemon::CONNECTION;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoService {
    account: Account,
}

impl TodoService {
    pub fn new(account: Account) -> Self {
        Self { account }
    }

    async fn fetch_config(&self) -> Result<HashMap<String, String>> {
        let registry = crate::daemon::REGISTRY
            .get()
            .ok_or_else(|| Error::Failed("Provider registry not loaded".to_string()))?;
        let manifest = registry
            .get(&self.account.provider)
            .ok_or_else(|| Error::Failed(format!("Unknown provider: {}", self.account.provider)))?;

        let connection = Connection::session().await?;
        let proxy = Provider1Proxy::new(&connection, manifest.provider.dbus_name.clone()).await?;

        proxy
            .get_service_config("todo")
            .await
            .map_err(|e| Error::Failed(format!("Provider did not return todo config: {e}")))
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Todo")]
impl TodoService {
    #[zbus(property)]
    async fn uri(&self) -> Result<String> {
        let config = self.fetch_config().await?;
        config
            .get("uri")
            .cloned()
            .ok_or_else(|| Error::Failed("Provider did not return a todo uri".to_string()))
    }

    #[zbus(property)]
    async fn accept_ssl_errors(&self) -> Result<bool> {
        let config = self.fetch_config().await?;
        Ok(config
            .get("accept_ssl_errors")
            .map(|v| v == "true")
            .unwrap_or(false))
    }
}

#[async_trait]
impl AccountService for TodoService {
    fn name(&self) -> &str {
        "Todo"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Todo"
    }

    fn is_supported(&self, account: &Account) -> bool {
        account.services.contains_key(&Service::Todo)
    }

    async fn get_config(&self, account: &Account) -> Result<ServiceConfig> {
        let config = self.fetch_config().await?;
        let mut settings = HashMap::new();
        for (key, value) in config {
            settings.insert(key, value.into());
        }

        Ok(ServiceConfig {
            service_type: "Todo".to_string(),
            provider_type: account.provider.clone(),
            settings,
        })
    }

    async fn add_service(&self) -> Result<bool> {
        tracing::info!(
            "Adding a todo service for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .at(
                    format!("/dev/edfloreshz/Accounts/Todo/{}", self.account.dbus_id()),
                    self.clone(),
                )
                .await?;
        }
        Ok(false)
    }

    async fn remove_service(&self) -> Result<bool> {
        tracing::info!(
            "Removing todo service for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .remove::<TodoService, String>(format!(
                    "/dev/edfloreshz/Accounts/Todo/{}",
                    self.account.dbus_id()
                ))
                .await?;
        }
        Ok(false)
    }

    async fn ensure_credentials(&self, _account: &mut Account) -> Result<()> {
        Ok(())
    }
}
