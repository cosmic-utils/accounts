pub use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
pub use uuid::Uuid;

pub mod zbus;

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
    pub credentials: Credentials,
    pub capabilities: Vec<Capability>,
}

impl From<zbus::Account> for Account {
    fn from(value: zbus::Account) -> Self {
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
            credentials: value.credentials.into(),
            capabilities: value.capabilities.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Deserialize)]
pub struct AccountProviderConfig {
    pub provider: AccountProvider,
}

#[derive(Deserialize)]
pub struct AccountProvider {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Provider {
    Google,
    Microsoft,
}

impl Provider {
    pub fn from_str(s: impl ToString) -> Option<Self> {
        match s.to_string().to_lowercase().as_str() {
            "google" => Some(Provider::Google),
            "microsoft" => Some(Provider::Microsoft),
            _ => None,
        }
    }

    pub fn list() -> [Self; 2] {
        [Self::Google, Self::Microsoft]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scope: Vec<String>,
    pub token_type: String,
}

impl From<zbus::Credentials> for Credentials {
    fn from(value: zbus::Credentials) -> Self {
        Credentials {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            expires_at: value
                .expires_at
                .map(|lu| DateTime::from_str(&lu).ok())
                .unwrap(),
            scope: value.scope,
            token_type: value.token_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Capability {
    Email,
    Calendar,
    Contacts,
    Todo,
}

impl From<String> for Capability {
    fn from(value: String) -> Self {
        match value.as_str() {
            "Email" => Capability::Email,
            "Calendar" => Capability::Calendar,
            "Contacts" => Capability::Contacts,
            "Todo" => Capability::Todo,
            capability => panic!("Invalid capability: {}", capability),
        }
    }
}

impl Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::Email => write!(f, "Email"),
            Capability::Calendar => write!(f, "Calendar"),
            Capability::Contacts => write!(f, "Contacts"),
            Capability::Todo => write!(f, "Todo"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Google => write!(f, "Google"),
            Provider::Microsoft => write!(f, "Microsoft"),
        }
    }
}

impl Provider {
    pub fn default_capabilities(&self) -> Vec<Capability> {
        match self {
            Provider::Google => vec![
                Capability::Email,
                Capability::Calendar,
                Capability::Contacts,
                Capability::Todo,
            ],
            Provider::Microsoft => vec![
                Capability::Email,
                Capability::Calendar,
                Capability::Contacts,
                Capability::Todo,
            ],
        }
    }
}
