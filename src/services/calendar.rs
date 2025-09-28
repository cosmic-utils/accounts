use std::collections::HashMap;

use async_trait::async_trait;
use zbus::{
    fdo::{Error, Result},
    interface,
};

use crate::{
    models::{Account, Capability, Provider},
    services::{Service, ServiceConfig},
};

pub struct CalendarService {
    account_id: String,
}

impl CalendarService {
    pub fn new(account_id: String) -> Self {
        Self { account_id }
    }
}

#[interface(name = "com.system76.CosmicAccounts.Calendar")]
impl CalendarService {
    /// CalDAV URI - matches GOA's Uri property exactly
    #[zbus(property)]
    async fn uri(&self) -> Result<String> {
        if self.account_id.contains("google") {
            Ok("https://apidata.googleusercontent.com/caldav/v2/".to_string())
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
impl Service for CalendarService {
    fn name(&self) -> &str {
        "Calendar"
    }

    fn interface_name(&self) -> &str {
        "com.system76.CosmicAccounts.Calendar"
    }

    fn is_supported(&self, account: &Account) -> bool {
        account.capabilities.contains(&Capability::Calendar)
    }

    async fn get_config(&self, account: &Account) -> Result<ServiceConfig> {
        let mut settings = HashMap::new();

        match account.provider {
            Provider::Google => {
                settings.insert(
                    "uri".to_string(),
                    "https://apidata.googleusercontent.com/caldav/v2/".into(),
                );
            }
            Provider::Microsoft => {
                settings.insert("uri".to_string(), "https://outlook.office365.com/".into());
            }
        }

        settings.insert("accept_ssl_errors".to_string(), false.into());

        Ok(ServiceConfig {
            service_type: "Calendar".to_string(),
            provider_type: account.provider.to_string(),
            settings,
        })
    }

    async fn ensure_credentials(&self, _account: &mut Account) -> Result<()> {
        Ok(())
    }
}
