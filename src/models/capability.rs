use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialOrd, Ord, PartialEq)]
pub enum Capability {
    Email,
    Calendar,
    Contacts,
    Todo,
}

impl Capability {
    pub fn from_str(value: String) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "email" => Some(Capability::Email),
            "calendar" => Some(Capability::Calendar),
            "contacts" => Some(Capability::Contacts),
            "todo" => Some(Capability::Todo),
            _ => None,
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
