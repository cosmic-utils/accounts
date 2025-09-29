use std::{collections::BTreeMap, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Capability, Provider};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub id: Uuid,
    pub provider: Provider,
    pub display_name: String,
    pub username: String,
    pub email: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub capabilities: BTreeMap<Capability, bool>,
}

use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

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
    pub capabilities: BTreeMap<String, bool>,
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
            capabilities: value
                .capabilities
                .iter()
                .map(|(capability, enabled)| (capability.to_string(), *enabled))
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
            capabilities: value
                .capabilities
                .iter()
                .map(|(capability, enabled)| (capability.to_string(), *enabled))
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
            capabilities: value
                .capabilities
                .into_iter()
                .map(|(capability, enabled)| (Capability::from_str(capability).unwrap(), enabled))
                .collect(),
        }
    }
}
