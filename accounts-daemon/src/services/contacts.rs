use std::collections::HashMap;

use async_trait::async_trait;
use zbus::{
    fdo::{Error, Result},
    interface,
};

use crate::{
    models::{Account, Provider, Service},
    services::{Service, ServiceConfig},
};

pub struct ContactsService {
    account_id: String,
}

impl ContactsService {
    pub fn new(account_id: String) -> Self {
        Self { account_id }
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Contacts")]
impl ContactsService {
    #[zbus(property)]
    async fn uri(&self) -> Result<String> {
        if self.account_id.contains("google") {
            Ok("https://www.googleapis.com/.well-known/carddav".to_string())
        } else if self.account_id.contains("microsoft") {
            Ok("https://outlook.office365.com/".to_string())
        } else {
            Err(Error::Failed("Unsupported provider".to_string()))
        }
    }

    /// Whether to accept SSL errors - matches GOA's AcceptSslErrors
    #[zbus(property)]
    async fn accept_ssl_errors(&self) -> Result<bool> {
        Ok(false)
    }
}

#[async_trait]
impl Service for ContactsService {
    fn name(&self) -> &str {
        "Contacts"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Contacts"
    }

    fn is_supported(&self, account: &Account) -> bool {
        account.services.contains_key(&Service::Contacts)
    }

    async fn get_config(&self, account: &Account) -> Result<ServiceConfig> {
        let mut settings = HashMap::new();

        match account.provider {
            Provider::Google => {
                settings.insert(
                    "uri".to_string(),
                    "https://www.googleapis.com/.well-known/carddav".into(),
                );
            }
            Provider::Microsoft => {
                settings.insert("uri".to_string(), "https://outlook.office365.com/".into());
            }
        }

        settings.insert("accept_ssl_errors".to_string(), false.into());

        Ok(ServiceConfig {
            service_type: "Contacts".to_string(),
            provider_type: account.provider.to_string(),
            settings,
        })
    }

    async fn ensure_credentials(&self, _account: &mut Account) -> Result<()> {
        Ok(())
    }
}
