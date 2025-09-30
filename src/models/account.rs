use std::{collections::BTreeMap, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

use crate::models::{Provider, Service};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Account {
    pub id: Uuid,
    pub provider: Provider,
    pub display_name: String,
    pub username: String,
    pub email: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub services: BTreeMap<Service, bool>,
}

impl Account {
    pub fn dbus_id(&self) -> String {
        self.id.to_string().replace("-", "_")
    }
}

#[derive(Debug, Clone, PartialEq, DeserializeDict, SerializeDict, Type)]
#[zvariant(signature = "dict")]
pub struct DbusAccount {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub username: String,
    pub email: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub last_used: Option<String>,
    pub services: BTreeMap<String, bool>,
}

impl From<Account> for DbusAccount {
    fn from(value: Account) -> Self {
        Self {
            id: value.id.to_string(),
            provider: value.provider.to_string(),
            display_name: value.display_name,
            username: value.username,
            email: value.email,
            enabled: value.enabled,
            created_at: value.created_at.to_string(),
            last_used: value
                .last_used
                .clone()
                .map(|last_used| last_used.to_string()),
            services: value
                .services
                .iter()
                .map(|(service, enabled)| (service.to_string(), *enabled))
                .collect(),
        }
    }
}

impl From<&Account> for DbusAccount {
    fn from(value: &Account) -> Self {
        Self {
            id: value.id.to_string(),
            provider: value.provider.to_string(),
            display_name: value.display_name.clone(),
            username: value.username.clone(),
            email: value.email.clone(),
            enabled: value.enabled,
            created_at: value.created_at.to_string(),
            last_used: value
                .last_used
                .clone()
                .map(|last_used| last_used.to_string()),
            services: value
                .services
                .iter()
                .map(|(service, enabled)| (service.to_string(), *enabled))
                .collect(),
        }
    }
}

impl From<DbusAccount> for Account {
    fn from(value: DbusAccount) -> Self {
        Account {
            id: Uuid::from_str(&value.id).unwrap(),
            provider: Provider::from_str(&value.provider).unwrap(),
            display_name: value.display_name,
            username: value.username,
            email: value.email,
            enabled: value.enabled,
            created_at: DateTime::from_str(&value.created_at).unwrap(),
            last_used: value
                .last_used
                .map(|lu| DateTime::from_str(&lu).ok())
                .unwrap(),
            services: value
                .services
                .into_iter()
                .map(|(service, enabled)| (Service::from_str(service).unwrap(), enabled))
                .collect(),
        }
    }
}
