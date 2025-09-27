mod calendar;
pub use calendar::*;
mod contacts;
pub use contacts::*;
mod mail;
pub use mail::*;
mod factory;
pub use factory::*;
mod todo;
pub use todo::*;

use crate::Account;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zbus::fdo::Result;

/// Trait that all service implementations must implement
#[async_trait]
pub trait Service: Send + Sync {
    /// Get the service name (e.g., "Mail", "Calendar")
    fn name(&self) -> &str;

    /// Get the D-Bus interface name for this service
    fn interface_name(&self) -> &str;

    /// Check if this service is supported by the account
    fn is_supported(&self, account: &Account) -> bool;

    /// Get service-specific configuration
    async fn get_config(&self, account: &Account) -> Result<ServiceConfig>;

    /// Ensure credentials are valid for this service
    async fn ensure_credentials(&self, account: &mut Account) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_type: String,
    pub provider_type: String,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Service capabilities that match GOA's service types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServiceCapability {
    Mail,
    Calendar,
    Contacts,
    Files,
    Documents,
    Photos,
    Chat,
    VideoCall,
    ToDo,
    Maps,
    Music,
    ReadLater,
    Ticketing,
    Printers,
}

impl ServiceCapability {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Mail,
            Self::Calendar,
            Self::Contacts,
            Self::Files,
            Self::Documents,
            Self::Photos,
            Self::Chat,
            Self::VideoCall,
            Self::ToDo,
            Self::Maps,
            Self::Music,
            Self::ReadLater,
            Self::Ticketing,
            Self::Printers,
        ]
    }
}
