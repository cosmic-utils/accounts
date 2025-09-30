use std::collections::HashMap;

use async_trait::async_trait;
use zbus::{
    fdo::{Error, Result},
    interface,
};

use crate::{
    models::{Account, Provider},
    services::{Service, ServiceConfig},
};

pub struct TodoService {
    account_id: String,
}

impl TodoService {
    pub fn new(account_id: String) -> Self {
        Self { account_id }
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Todo")]
impl TodoService {
    /// ToDo API URI - following GOA's Uri pattern
    #[zbus(property)]
    async fn uri(&self) -> Result<String> {
        if self.account_id.contains("google") {
            Ok("https://tasks.googleapis.com/tasks/v1/".to_string())
        } else if self.account_id.contains("microsoft") {
            Ok("https://graph.microsoft.com/v1.0/me/todo".to_string())
        } else {
            Err(Error::Failed("Unsupported provider".to_string()))
        }
    }
}

#[async_trait]
impl Service for TodoService {
    fn name(&self) -> &str {
        "Todo"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Todo"
    }

    fn is_supported(&self, account: &Account) -> bool {
        // Check if the account has todo services
        matches!(account.provider, Provider::Google | Provider::Microsoft)
    }

    async fn get_config(&self, account: &Account) -> Result<ServiceConfig> {
        let mut settings = HashMap::new();

        match account.provider {
            Provider::Google => {
                settings.insert(
                    "uri".to_string(),
                    "https://tasks.googleapis.com/tasks/v1/".into(),
                );
            }
            Provider::Microsoft => {
                settings.insert(
                    "uri".to_string(),
                    "https://graph.microsoft.com/v1.0/me/todo".into(),
                );
            }
        }

        Ok(ServiceConfig {
            service_type: "Todo".to_string(),
            provider_type: account.provider.to_string(),
            settings,
        })
    }

    async fn ensure_credentials(&self, _account: &mut Account) -> Result<()> {
        Ok(())
    }
}
