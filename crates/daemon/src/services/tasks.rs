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
pub struct TasksService {
    account: Account,
}

impl TasksService {
    pub fn new(account: Account) -> Self {
        Self { account }
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Endpoint.Tasks")]
impl TasksService {
    /// CalDAV collection URL for VTODO components; may equal the calendar URL
    /// for providers that don't separate them.
    #[zbus(property)]
    async fn uri(&self) -> Result<String> {
        let manifest = provider_manifest(&self.account)?;
        let endpoint = manifest.endpoint.tasks.as_ref().ok_or_else(|| {
            Error::Failed(format!(
                "Provider {} has no tasks endpoint",
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
impl AccountService for TasksService {
    fn name(&self) -> &str {
        "Tasks"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Endpoint.Tasks"
    }

    fn is_supported(&self, account: &Account) -> bool {
        account.services.contains_key(&Service::Tasks)
    }

    async fn add_service(&self) -> Result<bool> {
        tracing::info!(
            "Adding the tasks endpoint for account {}",
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
            "Removing the tasks endpoint for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .remove::<TasksService, String>(endpoint_object_path(&self.account))
                .await?;
        }
        Ok(false)
    }

    async fn ensure_credentials(&self, account: &mut Account) -> Result<()> {
        refresh_account_credentials(account).await
    }
}
