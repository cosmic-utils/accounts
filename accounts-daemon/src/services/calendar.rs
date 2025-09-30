use std::collections::HashMap;

use accounts::{
    AccountService, ServiceConfig,
    models::{Account, Provider, Service},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use zbus::{
    fdo::{Error, Result},
    interface,
};

use crate::CONNECTION;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalendarService {
    account: Account,
}

impl CalendarService {
    pub fn new(account: Account) -> Self {
        Self { account }
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Calendar")]
impl CalendarService {
    #[zbus(property)]
    async fn uri(&self) -> Result<String> {
        if self.account.provider == Provider::Google {
            Ok("https://apidata.googleusercontent.com/caldav/v2/".to_string())
        } else if self.account.provider == Provider::Microsoft {
            Ok("https://outlook.office365.com/".to_string())
        } else {
            Err(Error::Failed("Unsupported provider".to_string()))
        }
    }

    #[zbus(property)]
    async fn accept_ssl_errors(&self) -> Result<bool> {
        Ok(false)
    }
}

#[async_trait]
impl AccountService for CalendarService {
    fn name(&self) -> &str {
        "Calendar"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Calendar"
    }

    fn is_supported(&self, account: &Account) -> bool {
        account.services.contains_key(&Service::Calendar)
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

    async fn add_service(&self) -> Result<bool> {
        tracing::info!(
            "Adding a calendar service for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .at(
                    format!(
                        "/dev/edfloreshz/Accounts/Calendar/{}",
                        self.account.dbus_id()
                    ),
                    self.clone(),
                )
                .await?;
        }
        Ok(false)
    }

    async fn remove_service(&self) -> Result<bool> {
        tracing::info!(
            "Removing calendar service for account {}",
            self.account.dbus_id()
        );
        if let Some(connection) = CONNECTION.get() {
            connection
                .object_server()
                .remove::<CalendarService, String>(format!(
                    "/dev/edfloreshz/Accounts/Calendar/{}",
                    self.account.dbus_id()
                ))
                .await?;
        }
        Ok(false)
    }

    async fn ensure_credentials(&self, _account: &mut Account) -> Result<()> {
        Ok(())
    }
}
