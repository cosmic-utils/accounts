use std::collections::HashMap;

use accounts::{
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

use crate::CONNECTION;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalendarService {
    account: Account,
}

impl CalendarService {
    pub fn new(account: Account) -> Self {
        Self { account }
    }

    /// Connection info for this account's calendar service comes from the
    /// account's provider process, not from anything hardcoded here.
    async fn fetch_config(&self) -> Result<HashMap<String, String>> {
        let registry = crate::REGISTRY
            .get()
            .ok_or_else(|| Error::Failed("Provider registry not loaded".to_string()))?;
        let manifest = registry
            .get(&self.account.provider)
            .ok_or_else(|| Error::Failed(format!("Unknown provider: {}", self.account.provider)))?;

        let connection = Connection::session().await?;
        let proxy = Provider1Proxy::new(&connection, manifest.provider.dbus_name.clone()).await?;

        proxy
            .get_service_config("calendar")
            .await
            .map_err(|e| Error::Failed(format!("Provider did not return calendar config: {e}")))
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Calendar")]
impl CalendarService {
    #[zbus(property)]
    async fn uri(&self) -> Result<String> {
        let config = self.fetch_config().await?;
        config
            .get("uri")
            .cloned()
            .ok_or_else(|| Error::Failed("Provider did not return a calendar uri".to_string()))
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
impl AccountService for CalendarService {
    fn name(&self) -> &str {
        "Calendar"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Calendar"
    }

    fn is_supported(&self, account: &Account) -> bool {
        account.services.contains_key(&Service::Calendar)
    }

    async fn get_config(&self, account: &Account) -> Result<ServiceConfig> {
        let config = self.fetch_config().await?;
        let mut settings = HashMap::new();
        for (key, value) in config {
            settings.insert(key, value.into());
        }

        Ok(ServiceConfig {
            service_type: "Calendar".to_string(),
            provider_type: account.provider.clone(),
            settings,
        })
    }

    async fn add_service(&self) -> Result<bool> {
        tracing::info!(
            "Adding a calendar service for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .at(
                    format!(
                        "/dev/edfloreshz/Accounts/Calendar/{}",
                        self.account.dbus_id()
                    ),
                    self.clone(),
                )
                .await?;
        }
        Ok(false)
    }

    async fn remove_service(&self) -> Result<bool> {
        tracing::info!(
            "Removing calendar service for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .remove::<CalendarService, String>(format!(
                    "/dev/edfloreshz/Accounts/Calendar/{}",
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
