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
pub struct MailService {
    account: Account,
}

impl MailService {
    pub fn new(account: Account) -> Self {
        Self { account }
    }

    /// IMAP/SMTP host and protocol settings come from the account's provider
    /// process. Identity fields (email address, display name) are already
    /// known locally from the account itself and don't need a round trip.
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
    /// Email address - matches GOA's EmailAddress property
    #[zbus(property)]
    async fn email_address(&self) -> Result<String> {
        Ok(self.account.email.clone().unwrap_or_default())
    }

    /// Display name - matches GOA's Name property
    #[zbus(property)]
    async fn name(&self) -> Result<String> {
        Ok(self.account.display_name.clone())
    }

    // IMAP Properties - matching GOA exactly

    /// IMAP hostname - matches GOA's ImapHost
    #[zbus(property)]
    async fn imap_host(&self) -> Result<String> {
        Self::string_setting(&self.fetch_config().await?, "imap_host")
    }

    /// IMAP username - matches GOA's ImapUserName
    #[zbus(property)]
    async fn imap_user_name(&self) -> Result<String> {
        self.email_address().await
    }

    /// Whether IMAP is supported - matches GOA's ImapSupported
    #[zbus(property)]
    async fn imap_supported(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "imap_supported",
            true,
        ))
    }

    /// Whether IMAP uses SSL - matches GOA's ImapUseSsl
    #[zbus(property)]
    async fn imap_use_ssl(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "imap_use_ssl",
            true,
        ))
    }

    /// Whether IMAP uses TLS - matches GOA's ImapUseTls
    #[zbus(property)]
    async fn imap_use_tls(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "imap_use_tls",
            false,
        ))
    }

    /// Whether to accept SSL errors - matches GOA's ImapAcceptSslErrors
    #[zbus(property)]
    async fn imap_accept_ssl_errors(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "imap_accept_ssl_errors",
            false,
        ))
    }

    // SMTP Properties - matching GOA exactly

    /// SMTP hostname - matches GOA's SmtpHost
    #[zbus(property)]
    async fn smtp_host(&self) -> Result<String> {
        Self::string_setting(&self.fetch_config().await?, "smtp_host")
    }

    /// SMTP username - matches GOA's SmtpUserName
    #[zbus(property)]
    async fn smtp_user_name(&self) -> Result<String> {
        self.email_address().await
    }

    /// Whether SMTP is supported - matches GOA's SmtpSupported
    #[zbus(property)]
    async fn smtp_supported(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_supported",
            true,
        ))
    }

    /// Whether SMTP uses authentication - matches GOA's SmtpUseAuth
    #[zbus(property)]
    async fn smtp_use_auth(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_use_auth",
            true,
        ))
    }

    /// Whether SMTP uses SSL - matches GOA's SmtpUseSsl
    #[zbus(property)]
    async fn smtp_use_ssl(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_use_ssl",
            false,
        ))
    }

    /// Whether SMTP uses TLS - matches GOA's SmtpUseTls
    #[zbus(property)]
    async fn smtp_use_tls(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_use_tls",
            true,
        ))
    }

    /// Whether to accept SMTP SSL errors - matches GOA's SmtpAcceptSslErrors
    #[zbus(property)]
    async fn smtp_accept_ssl_errors(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_accept_ssl_errors",
            false,
        ))
    }

    /// SMTP supports LOGIN auth - matches GOA's SmtpAuthLogin
    #[zbus(property)]
    async fn smtp_auth_login(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_auth_login",
            false,
        ))
    }

    /// SMTP supports PLAIN auth - matches GOA's SmtpAuthPlain
    #[zbus(property)]
    async fn smtp_auth_plain(&self) -> Result<bool> {
        Ok(Self::bool_setting(
            &self.fetch_config().await?,
            "smtp_auth_plain",
            false,
        ))
    }

    /// SMTP supports XOAUTH2 auth - matches GOA's SmtpAuthXoauth2
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
