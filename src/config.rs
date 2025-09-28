use crate::models::{Account, Provider};
use cosmic_config::{
    self, cosmic_config_derive::CosmicConfigEntry, Config, CosmicConfigEntry, Error,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize, CosmicConfigEntry)]
pub struct CosmicAccountsConfig {
    pub accounts: Vec<Account>,
}

impl CosmicAccountsConfig {
    pub fn config_handler() -> Option<Config> {
        Config::new("dev.edfloreshz.CosmicAccountsDaemon", CONFIG_VERSION).ok()
    }

    pub fn config() -> CosmicAccountsConfig {
        match Self::config_handler() {
            Some(config_handler) => CosmicAccountsConfig::get_entry(&config_handler)
                .unwrap_or_else(|(errs, config)| {
                    tracing::info!("errors loading config: {:?}", errs);
                    config
                }),
            None => CosmicAccountsConfig::default(),
        }
    }

    pub fn save_account(&mut self, account: &Account) -> Result<(), Error> {
        let mut accounts = self.accounts.clone();
        accounts.push(account.clone());
        if let Some(handler) = Self::config_handler() {
            self.set_accounts(&handler, accounts)?;
        } else {
            tracing::warn!("No config handler available, account not saved");
        }
        Ok(())
    }

    pub fn remove_account(&mut self, id: &Uuid) -> Result<(), Error> {
        let mut accounts = self.accounts.clone();
        accounts.retain(|account| account.id != *id);
        if let Some(handler) = Self::config_handler() {
            self.set_accounts(&handler, accounts)?;
        } else {
            tracing::warn!("No config handler available, account not removed");
        }
        Ok(())
    }

    pub fn get_account(&self, id: &Uuid) -> Option<Account> {
        self.accounts.iter().find(|a| a.id == *id).cloned()
    }

    pub fn account_exists(&self, username: &String, provider: &Provider) -> bool {
        self.accounts
            .iter()
            .any(|a| a.username == *username && a.provider == *provider)
    }
}
