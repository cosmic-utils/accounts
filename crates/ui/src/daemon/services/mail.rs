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

    fn bool_setting(config: &HashMap<String, String>, key: &str, default: bool) -> bool {
        config.get(key).map(|v| v == "true").unwrap_or(default)
    }

    fn string_setting(config: &HashMap<String, String>, key: &str) -> Result<String> {
        config
            .get(key)
            .cloned()
            .ok_or_else(|| Error::Failed(format!("Provider did not return {key}")))
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Mail")]
impl MailService {
    #[zbus(property)]
    async fn email_address(&self) -> Result<String> {
        Ok(self.account.email.clone().unwrap_or_default())
    }

    #[zbus(property)]
    async fn name(&self) -> Result<String> {
        Ok(self.account.display_name.clone())
    }

    #[zbus(property)]
    async fn imap_host(&self) -> Result<String> {
        Self::string_setting(&self.fetch_config().await?, "imap_host")
    }

    #[zbus(property)]
    async fn imap_user_name(&self) -> Result<String> {
        self.email_address().await
    }

    #[zbus(property)]
    async fn imap_supported(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "imap_supported",
            true,
        ))
    }

    #[zbus(property)]
    async fn imap_use_ssl(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "imap_use_ssl",
            true,
        ))
    }

    #[zbus(property)]
    async fn imap_use_tls(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "imap_use_tls",
            false,
        ))
    }

    #[zbus(property)]
    async fn imap_accept_ssl_errors(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "imap_accept_ssl_errors",
            false,
        ))
    }

    #[zbus(property)]
    async fn smtp_host(&self) -> Result<String> {
        Self::string_setting(&self.fetch_config().await?, "smtp_host")
    }

    #[zbus(property)]
    async fn smtp_user_name(&self) -> Result<String> {
        self.email_address().await
    }

    #[zbus(property)]
    async fn smtp_supported(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_supported",
            true,
        ))
    }

    #[zbus(property)]
    async fn smtp_use_auth(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_use_auth",
            true,
        ))
    }

    #[zbus(property)]
    async fn smtp_use_ssl(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_use_ssl",
            false,
        ))
    }

    #[zbus(property)]
    async fn smtp_use_tls(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_use_tls",
            true,
        ))
    }

    #[zbus(property)]
    async fn smtp_accept_ssl_errors(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_accept_ssl_errors",
            false,
        ))
    }

    #[zbus(property)]
    async fn smtp_auth_login(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_auth_login",
            false,
        ))
    }

    #[zbus(property)]
    async fn smtp_auth_plain(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_auth_plain",
            false,
        ))
    }

    #[zbus(property)]
    async fn smtp_auth_xoauth2(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_auth_xoauth2",
            true,
        ))
    }
}

#[async_trait]
impl AccountService for MailService {
    fn name(&self) -> &str {
        "Mail"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Mail"
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

        if let Some(email) = &account.email {
            settings.insert("email_address".to_string(), email.clone().into());
            settings.insert("imap_user_name".to_string(), email.clone().into());
            settings.insert("smtp_user_name".to_string(), email.clone().into());
        }
        settings.insert("name".to_string(), account.display_name.clone().into());

        Ok(ServiceConfig {
            service_type: "Mail".to_string(),
            provider_type: account.provider.clone(),
            settings,
        })
    }

    async fn add_service(&self) -> Result<bool> {
        tracing::info!(
            "Adding a mail service for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .at(
                    format!("/dev/edfloreshz/Accounts/Mail/{}", self.account.dbus_id()),
                    self.clone(),
                )
                .await?;
        }
        Ok(false)
    }

    async fn remove_service(&self) -> Result<bool> {
        tracing::info!(
            "Removing mail service for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .remove::<MailService, String>(format!(
                    "/dev/edfloreshz/Accounts/Mail/{}",
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
