use std::collections::HashMap;

use crate::{Error, Result};
use accounts::models::Credential;
use secret_service::{EncryptionType, SecretService};
use uuid::Uuid;

pub struct CredentialStorage {
    service: SecretService<'static>,
}

impl CredentialStorage {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            service: SecretService::connect(EncryptionType::Dh)
                .await
                .map_err(Error::CredentialStorage)?,
        })
    }

    pub async fn get_account_credentials(&self, account_id: &Uuid) -> Result<Credential> {
        let search_items = self
            .service
            .search_items(HashMap::from([(
                "account_id",
                account_id.to_string().as_str(),
            )]))
            .await
            .map_err(Error::CredentialStorage)?;
        if let Some(item) = search_items.unlocked.first() {
            let secret_value = item.get_secret().await.map_err(Error::CredentialStorage)?;
            let serialized = std::str::from_utf8(&secret_value).map_err(Error::Utf8)?;
            let credential: Credential = serde_json::from_str(serialized)?;
            Ok(credential)
        } else {
            Err(Error::StorageError(format!(
                "Credentials not found for account {}",
                account_id
            )))
        }
    }

    pub async fn set_account_credentials(
        &self,
        account_id: &Uuid,
        credential: &Credential,
    ) -> Result<()> {
        let collection = self
            .service
            .get_default_collection()
            .await
            .map_err(Error::CredentialStorage)?;
        let serialized = serde_json::to_string(credential)?;

        collection
            .create_item(
                &format!("Account: {}", account_id),
                HashMap::from([("account_id", account_id.to_string().as_str())]),
                serialized.as_bytes(),
                true, // replace existing
                "text/plain",
            )
            .await
            .map_err(|e| Error::StorageError(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_account_credentials(&self, account_id: &Uuid) -> Result<()> {
        let collection = self
            .service
            .get_default_collection()
            .await
            .map_err(Error::CredentialStorage)?;

        let search_items = collection
            .search_items(HashMap::from([(
                "account_id",
                account_id.to_string().as_str(),
            )]))
            .await
            .map_err(Error::CredentialStorage)?;

        for item in search_items {
            item.delete().await.map_err(Error::CredentialStorage)?;
        }

        Ok(())
    }
}
