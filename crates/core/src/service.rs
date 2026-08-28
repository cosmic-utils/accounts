use std::collections::HashMap;

use crate::models::Account;
use async_trait::async_trait;
use zbus::{fdo::Result, zvariant::Value};

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub service_type: String,
    pub provider_type: String,
    pub settings: HashMap<String, Value<'static>>,
}

/// Trait that all service implementations must implement
#[async_trait]
pub trait AccountService: Send + Sync {
    /// Get the service name (e.g., "Mail", "Calendar")
    fn name(&self) -> &str;

    /// Get the D-Bus interface name for this service
    fn interface_name(&self) -> &str;

    /// Check if this service is supported by the account
    fn is_supported(&self, account: &Account) -> bool;

    /// Get service configuration for the given account
    async fn get_config(&self, account: &Account) -> Result<ServiceConfig>;

    /// Add the service to the object server
    async fn add_service(&self) -> Result<bool>;

    /// Remove the service from the object server
    async fn remove_service(&self) -> Result<bool>;

    /// Ensure credentials are valid for this service
    async fn ensure_credentials(&self, account: &mut Account) -> Result<()>;
}
