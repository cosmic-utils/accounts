use std::fmt::Display;

use serde::{Deserialize, Serialize};

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

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Google => write!(f, "Google"),
            Provider::Microsoft => write!(f, "Microsoft"),
        }
    }
}
