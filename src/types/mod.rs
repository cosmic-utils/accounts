use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Capability {
    Email,
    Calendar,
    Contacts,
    Files,
    Photos,
    Documents,
    Chat,
    VideoCall,
    Repository,
    Issues,
    PullRequests,
}

impl Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::Email => write!(f, "Email"),
            Capability::Calendar => write!(f, "Calendar"),
            Capability::Contacts => write!(f, "Contacts"),
            Capability::Files => write!(f, "Files"),
            Capability::Photos => write!(f, "Photos"),
            Capability::Documents => write!(f, "Documents"),
            Capability::Chat => write!(f, "Chat"),
            Capability::VideoCall => write!(f, "Video Call"),
            Capability::Repository => write!(f, "Repository"),
            Capability::Issues => write!(f, "Issues"),
            Capability::PullRequests => write!(f, "Pull Requests"),
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
                Capability::Files,
                Capability::Photos,
            ],
            Provider::Microsoft => vec![
                Capability::Email,
                Capability::Calendar,
                Capability::Contacts,
                Capability::Files,
                Capability::Documents,
            ],
        }
    }
}
