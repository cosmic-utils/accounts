use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Provider {
    Google,
    Microsoft,
    GitHub,
    GitLab,
    Facebook,
    Twitter,
    Custom {
        name: String,
        auth_url: String,
        token_url: String,
    },
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl Provider {
    pub fn display_name(&self) -> &str {
        match self {
            Provider::Google => "Google",
            Provider::Microsoft => "Microsoft",
            Provider::GitHub => "GitHub",
            Provider::GitLab => "GitLab",
            Provider::Facebook => "Facebook",
            Provider::Twitter => "Twitter",
            Provider::Custom { name, .. } => name,
        }
    }

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
            Provider::GitHub | Provider::GitLab => vec![
                Capability::Repository,
                Capability::Issues,
                Capability::PullRequests,
            ],
            _ => vec![],
        }
    }
}
