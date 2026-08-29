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
use crate::daemon::services::{endpoint_object_path, refresh_account_credentials};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalendarService {
    account: Account,
}

impl CalendarService {
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
            .get_service_config("calendar")
            .await
            .map_err(|e| Error::Failed(format!("Provider did not return calendar config: {e}")))
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Endpoint.Calendar")]
impl CalendarService {
    /// CalDAV collection/principal URL; the client discovers individual
    /// calendars underneath via a normal CalDAV PROPFIND.
    #[zbus(property)]
    async fn uri(&self) -> Result<String> {
        let config = self.fetch_config().await?;
        config
            .get("uri")
            .cloned()
            .ok_or_else(|| Error::Failed("Provider did not return a calendar uri".to_string()))
    }

    /// Mirrors `Credentials.AuthMethod`.
    #[zbus(property)]
    async fn auth_method(&self) -> Result<String> {
        Ok("oauth2".to_string())
    }
}

#[async_trait]
impl AccountService for CalendarService {
    fn name(&self) -> &str {
        "Calendar"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Endpoint.Calendar"
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
            "Adding the calendar endpoint for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .at(endpoint_object_path(&self.account), self.clone())
                .await?;
        }
        Ok(false)
    }

    async fn remove_service(&self) -> Result<bool> {
        tracing::info!(
            "Removing the calendar endpoint for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .remove::<CalendarService, String>(endpoint_object_path(&self.account))
                .await?;
        }
        Ok(false)
    }

    async fn ensure_credentials(&self, account: &mut Account) -> Result<()> {
        refresh_account_credentials(account).await
    }
}
