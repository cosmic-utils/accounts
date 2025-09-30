use std::fmt::Display;

use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialOrd, Ord, PartialEq)]
pub enum Service {
    Email,
    Calendar,
    Contacts,
    Todo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum DbusService {
    Email,
    Calendar,
    Contacts,
    Todo,
}

impl Service {
    pub fn from_str(value: String) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "email" => Some(Service::Email),
            "calendar" => Some(Service::Calendar),
            "contacts" => Some(Service::Contacts),
            "todo" => Some(Service::Todo),
            _ => None,
        }
    }
}

impl Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Service::Email => write!(f, "Email"),
            Service::Calendar => write!(f, "Calendar"),
            Service::Contacts => write!(f, "Contacts"),
            Service::Todo => write!(f, "Todo"),
        }
    }
}

impl From<DbusService> for Service {
    fn from(value: DbusService) -> Self {
        match value {
            DbusService::Email => Service::Email,
            DbusService::Calendar => Service::Calendar,
            DbusService::Contacts => Service::Contacts,
            DbusService::Todo => Service::Todo,
        }
    }
}

impl From<Service> for DbusService {
    fn from(value: Service) -> Self {
        match value {
            Service::Email => DbusService::Email,
            Service::Calendar => DbusService::Calendar,
            Service::Contacts => DbusService::Contacts,
            Service::Todo => DbusService::Todo,
        }
    }
}

impl From<Service> for String {
    fn from(value: Service) -> Self {
        match value {
            Service::Email => "Email".to_string(),
            Service::Calendar => "Calendar".to_string(),
            Service::Contacts => "Contacts".to_string(),
            Service::Todo => "Todo".to_string(),
        }
    }
}
