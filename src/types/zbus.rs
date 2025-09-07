use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

#[derive(Debug, Clone, PartialEq, DeserializeDict, SerializeDict, Type)]
#[zvariant(signature = "dict")]
pub struct Account {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub username: String,
    pub email: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub last_used: Option<String>,
    pub credentials: Credentials,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, DeserializeDict, SerializeDict, Type)]
#[zvariant(signature = "dict")]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub scope: Vec<String>,
    pub token_type: String,
}

impl From<super::Account> for Account {
    fn from(value: super::Account) -> Self {
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
            credentials: Credentials {
                access_token: value.credentials.access_token,
                refresh_token: value.credentials.refresh_token,
                expires_at: value
                    .credentials
                    .expires_at
                    .map(|expires_at| expires_at.to_string()),
                scope: value.credentials.scope,
                token_type: value.credentials.token_type,
            },
            capabilities: value
                .capabilities
                .iter()
                .map(|capability| capability.to_string())
                .collect(),
        }
    }
}

impl From<&super::Account> for Account {
    fn from(value: &super::Account) -> Self {
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
            credentials: Credentials {
                access_token: value.credentials.access_token.clone(),
                refresh_token: value.credentials.refresh_token.clone(),
                expires_at: value
                    .credentials
                    .expires_at
                    .map(|expires_at| expires_at.to_string()),
                scope: value.credentials.scope.clone(),
                token_type: value.credentials.token_type.clone(),
            },
            capabilities: value
                .capabilities
                .iter()
                .map(|capability| capability.to_string())
                .collect(),
        }
    }
}
