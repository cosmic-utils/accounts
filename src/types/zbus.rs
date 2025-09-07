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
    pub capabilities: Vec<String>,
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
            capabilities: value
                .capabilities
                .iter()
                .map(|capability| capability.to_string())
                .collect(),
        }
    }
}
