use crate::models::Account;
use async_trait::async_trait;
use zbus::fdo::Result;

/// Trait that all service endpoint implementations must implement.
#[async_trait]
pub trait AccountService: Send + Sync {
    /// The service name (e.g. "Mail", "Calendar").
    fn name(&self) -> &str;

    /// The D-Bus interface name for this endpoint.
    fn interface_name(&self) -> &str;

    /// Whether this service is supported by the account.
    fn is_supported(&self, account: &Account) -> bool;

    /// Add the endpoint interface to the object server.
    async fn add_service(&self) -> Result<bool>;

    /// Remove the endpoint interface from the object server.
    async fn remove_service(&self) -> Result<bool>;

    /// Refresh the account's stored credentials if they have expired.
    async fn ensure_credentials(&self, account: &mut Account) -> Result<()>;
}
