use cosmic_accounts::models::Credential;
use keyring::Entry;
use uuid::Uuid;

use crate::Error;

const SERVICE_NAME: &str = "cosmic-accounts";

pub struct CredentialStorage;

impl CredentialStorage {
    pub fn new() -> Self {
        Self
    }

    pub fn get_account_credentials(&self, account_id: &Uuid) -> Result<Credential, Error> {
        let entry = Entry::new(SERVICE_NAME, &account_id.to_string())?;
        match entry.get_password() {
            Ok(serialized) => {
                let credential: Credential = serde_json::from_str(&serialized)?;
                Ok(credential)
            }
            Err(e) => Err(crate::Error::CredentialStorage(e)),
        }
    }

    pub fn set_account_credentials(
        &self,
        account_id: &Uuid,
        credential: &Credential,
    ) -> Result<(), Error> {
        let entry = Entry::new(SERVICE_NAME, &account_id.to_string())?;
        let serialized = serde_json::to_string(credential)?;
        entry.set_password(&serialized)?;
        Ok(())
    }

    pub fn delete_account_credentials(&self, account_id: &Uuid) -> Result<(), Error> {
        let entry = Entry::new(SERVICE_NAME, &account_id.to_string())?;
        entry.delete_password()?;
        Ok(())
    }

    pub fn update_account_credentials(
        &self,
        account_id: &Uuid,
        credential: &Credential,
    ) -> Result<(), Error> {
        self.delete_account_credentials(account_id)?;
        self.set_account_credentials(account_id, credential)
    }
}
