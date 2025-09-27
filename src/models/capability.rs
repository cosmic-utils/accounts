use std::fmt::Display;

use serde::{Deserialize, Serialize};

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
