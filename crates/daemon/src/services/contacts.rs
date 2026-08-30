use accounts_core::{
    AccountService,
    models::{Account, Service},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use zbus::{
    fdo::{Error, Result},
    interface,
};

use crate::CONNECTION;
use crate::services::{
    account_identity, endpoint_object_path, provider_manifest, refresh_account_credentials,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactsService {
    account: Account,
}

impl ContactsService {
    pub fn new(account: Account) -> Self {
        Self { account }
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Endpoint.Contacts")]
impl ContactsService {
    /// CardDAV collection/principal URL.
    #[zbus(property)]
    async fn uri(&self) -> Result<String> {
        let manifest = provider_manifest(&self.account)?;
        let endpoint = manifest.endpoint.contacts.as_ref().ok_or_else(|| {
            Error::Failed(format!(
                "Provider {} has no contacts endpoint",
                self.account.provider
            ))
        })?;
        Ok(endpoint.resolve(&account_identity(&self.account)))
    }

    /// Mirrors `Credentials.AuthMethod`.
    #[zbus(property)]
    async fn auth_method(&self) -> Result<String> {
        Ok("oauth2".to_string())
    }
}

#[async_trait]
impl AccountService for ContactsService {
    fn name(&self) -> &str {
        "Contacts"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Endpoint.Contacts"
    }

    fn is_supported(&self, account: &Account) -> bool {
        account.services.contains_key(&Service::Contacts)
    }

    async fn add_service(&self) -> Result<bool> {
        tracing::info!(
            "Adding the contacts endpoint for account {}",
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
            "Removing the contacts endpoint for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .remove::<ContactsService, String>(endpoint_object_path(&self.account))
                .await?;
        }
        Ok(false)
    }

    async fn ensure_credentials(&self, account: &mut Account) -> Result<()> {
        refresh_account_credentials(account).await
    }
}
