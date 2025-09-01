use crate::{Account, AccountsError, Result};
use keyring::{Entry, Error as KeyringError};
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

const SERVICE_NAME: &str = "cosmic-accounts";
const ACCOUNTS_KEY: &str = "accounts";

pub struct AccountStorage {
    entry: Entry,
}

impl AccountStorage {
    pub fn new() -> Result<Self> {
        let entry = Entry::new(SERVICE_NAME, ACCOUNTS_KEY)?;
        Ok(Self { entry })
    }

    pub fn save_accounts(&self, accounts: &HashMap<Uuid, Account>) -> Result<()> {
        let serialized = serde_json::to_string(accounts)?;
        self.entry.set_password(&serialized)?;
        Ok(())
    }

    pub fn load_accounts(&self) -> Result<HashMap<Uuid, Account>> {
        match self.entry.get_password() {
            Ok(serialized) => {
                let accounts: HashMap<Uuid, Account> = serde_json::from_str(&serialized)?;
                Ok(accounts)
            }
            Err(KeyringError::NoEntry) => Ok(HashMap::new()),
            Err(e) => Err(AccountsError::Storage(e)),
        }
    }

    pub fn save_account(&self, account: &Account) -> Result<()> {
        let mut accounts = self.load_accounts()?;
        accounts.insert(account.id, account.clone());
        self.save_accounts(&accounts)
    }

    pub fn remove_account(&self, id: &Uuid) -> Result<()> {
        let mut accounts = self.load_accounts()?;
        if accounts.remove(id).is_none() {
            return Err(AccountsError::AccountNotFound { id: id.to_string() });
        }
        self.save_accounts(&accounts)
    }

    pub fn get_account(&self, id: &Uuid) -> Result<Option<Account>> {
        let accounts = self.load_accounts()?;
        Ok(accounts.get(id).cloned())
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let accounts = self.load_accounts()?;
        Ok(accounts.into_values().collect())
    }
}
