use accounts_core::{
    AccountService,
    models::{Account, Service},
    registry::MailEndpointManifest,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use zbus::{
    fdo::{Error, Result},
    interface,
};

use crate::daemon::CONNECTION;
use crate::daemon::services::{
    endpoint_object_path, provider_manifest, refresh_account_credentials,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MailService {
    account: Account,
}

impl MailService {
    pub fn new(account: Account) -> Self {
        Self { account }
    }

    fn endpoint(&self) -> Result<&'static MailEndpointManifest> {
        provider_manifest(&self.account)?
            .endpoint
            .mail
            .as_ref()
            .ok_or_else(|| {
                Error::Failed(format!(
                    "Provider {} has no mail endpoint",
                    self.account.provider
                ))
            })
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Endpoint.Mail")]
impl MailService {
    #[zbus(property)]
    async fn imap_host(&self) -> Result<String> {
        Ok(self.endpoint()?.imap_host.clone())
    }

    #[zbus(property)]
    async fn imap_port(&self) -> Result<u16> {
        Ok(self.endpoint()?.imap_port)
    }

    #[zbus(property)]
    async fn smtp_host(&self) -> Result<String> {
        Ok(self.endpoint()?.smtp_host.clone())
    }

    #[zbus(property)]
    async fn smtp_port(&self) -> Result<u16> {
        Ok(self.endpoint()?.smtp_port)
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
