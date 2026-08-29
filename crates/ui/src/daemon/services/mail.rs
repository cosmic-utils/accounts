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

const DEFAULT_IMAP_PORT: u16 = 993;
const DEFAULT_SMTP_PORT: u16 = 587;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MailService {
    account: Account,
}

impl MailService {
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
            .get_service_config("email")
            .await
            .map_err(|e| Error::Failed(format!("Provider did not return email config: {e}")))
    }

    fn string_setting(config: &HashMap<String, String>, key: &str) -> Result<String> {
        config
            .get(key)
            .cloned()
            .ok_or_else(|| Error::Failed(format!("Provider did not return {key}")))
    }

    fn port_setting(config: &HashMap<String, String>, key: &str, default: u16) -> u16 {
        config
            .get(key)
            .and_then(|value| value.parse().ok())
            .unwrap_or(default)
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Endpoint.Mail")]
impl MailService {
    #[zbus(property)]
    async fn imap_host(&self) -> Result<String> {
        Self::string_setting(&self.fetch_config().await?, "imap_host")
    }

    #[zbus(property)]
    async fn imap_port(&self) -> Result<u16> {
        Ok(Self::port_setting(
            &self.fetch_config().await?,
            "imap_port",
            DEFAULT_IMAP_PORT,
        ))
    }

    #[zbus(property)]
    async fn smtp_host(&self) -> Result<String> {
        Self::string_setting(&self.fetch_config().await?, "smtp_host")
    }

    #[zbus(property)]
    async fn smtp_port(&self) -> Result<u16> {
        Ok(Self::port_setting(
            &self.fetch_config().await?,
            "smtp_port",
            DEFAULT_SMTP_PORT,
        ))
    }

    /// Mirrors `Credentials.AuthMethod`.
    #[zbus(property)]
    async fn auth_method(&self) -> Result<String> {
        Ok("oauth2".to_string())
    }
}

#[async_trait]
impl AccountService for MailService {
    fn name(&self) -> &str {
        "Mail"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Endpoint.Mail"
    }

    fn is_supported(&self, account: &Account) -> bool {
        account.services.contains_key(&Service::Email)
    }

    async fn get_config(&self, account: &Account) -> Result<ServiceConfig> {
        let config = self.fetch_config().await?;
        let mut settings = HashMap::new();
        for (key, value) in config {
            settings.insert(key, value.into());
        }

        Ok(ServiceConfig {
            service_type: "Mail".to_string(),
            provider_type: account.provider.clone(),
            settings,
        })
    }

    async fn add_service(&self) -> Result<bool> {
        tracing::info!(
            "Adding the mail endpoint for account {}",
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
            "Removing the mail endpoint for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .remove::<MailService, String>(endpoint_object_path(&self.account))
                .await?;
        }
        Ok(false)
    }

    async fn ensure_credentials(&self, account: &mut Account) -> Result<()> {
        refresh_account_credentials(account).await
    }
}
