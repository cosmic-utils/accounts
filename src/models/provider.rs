use std::{collections::BTreeMap, fmt::Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    pub fn file_name(&self) -> &str {
        match self {
            Provider::Google => "google.toml",
            Provider::Microsoft => "microsoft.toml",
        }
    }

    pub fn services(&self) -> BTreeMap<super::Service, bool> {
        match self {
            Provider::Google => BTreeMap::from([
                (super::Service::Email, false),
                (super::Service::Calendar, false),
            ]),
            Provider::Microsoft => BTreeMap::from([
                (super::Service::Email, false),
                (super::Service::Calendar, false),
            ]),
        }
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Google => write!(f, "Google"),
            Provider::Microsoft => write!(f, "Microsoft"),
        }
    }
}
