use crate::daemon::{Error, auth::AuthManager, manager::create_request, services::ServiceFactory};
use accounts_core::{config::AccountsConfig, models::Service};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use zbus::{fdo::Result, interface, object_server::SignalEmitter, zvariant::OwnedObjectPath};

/// Per-account D-Bus object served at `/dev/edfloreshz/Accounts/Accounts/<dbus_id>`.
///
/// State is shared with the rest of the daemon through the same `Arc<Mutex<..>>` handles
/// used by the `Manager` object, so every instance stays in sync with the on-disk config.
pub struct AccountInterface {
    pub(crate) id: Uuid,
    pub(crate) config: Arc<Mutex<AccountsConfig>>,
    pub(crate) auth_manager: Arc<Mutex<AuthManager>>,
}

impl AccountInterface {
    pub fn new(
        id: Uuid,
        config: Arc<Mutex<AccountsConfig>>,
        auth_manager: Arc<Mutex<AuthManager>>,
    ) -> Self {
        Self {
            id,
            config,
            auth_manager,
        }
    }

    async fn current(&self) -> Result<accounts_core::models::Account> {
        let config = self.config.lock().await;
        config
            .get_account(&self.id)
            .ok_or_else(|| Error::AccountNotFound(self.id.to_string()).into())
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Account")]
impl AccountInterface {
    #[zbus(property)]
    async fn id(&self) -> Result<String> {
        Ok(self.id.to_string())
    }

    #[zbus(property)]
    async fn provider_id(&self) -> Result<String> {
        Ok(self.current().await?.provider)
    }

    #[zbus(property)]
    async fn display_name(&self) -> Result<String> {
        Ok(self.current().await?.display_name)
    }

    #[zbus(property)]
    async fn set_display_name(&self, value: String) -> Result<()> {
        let mut account = self.current().await?;
        account.display_name = value;
        let mut config = self.config.lock().await;
        config
            .save_account(&account)
            .map_err(|e| zbus::fdo::Error::Failed(format!("Failed to save account: {e}")))
    }

    #[zbus(property)]
    async fn identity(&self) -> Result<String> {
        let account = self.current().await?;
        Ok(account.email.unwrap_or(account.username))
    }

    #[zbus(property)]
    async fn enabled(&self) -> Result<bool> {
        Ok(self.current().await?.enabled)
    }

    #[zbus(property)]
    async fn set_enabled(&self, value: bool) -> Result<()> {
        let mut account = self.current().await?;
        account.enabled = value;
        let mut config = self.config.lock().await;
        config
            .save_account(&account)
            .map_err(|e| zbus::fdo::Error::Failed(format!("Failed to save account: {e}")))
    }

    #[zbus(property)]
    async fn available_services(&self) -> Result<Vec<String>> {
        Ok(self
            .current()
            .await?
            .services
            .keys()
            .map(ToString::to_string)
            .collect())
    }

    #[zbus(property)]
    async fn enabled_services(&self) -> Result<Vec<String>> {
        Ok(self
            .current()
            .await?
            .services
            .iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(service, _)| service.to_string())
            .collect())
    }

    #[zbus(property)]
    async fn created_at(&self) -> Result<String> {
        Ok(self.current().await?.created_at.to_string())
    }

    #[zbus(property)]
    async fn last_used(&self) -> Result<String> {
        Ok(self
            .current()
            .await?
            .last_used
            .map(|last_used| last_used.to_string())
            .unwrap_or_default())
    }

    #[zbus(property)]
    async fn email(&self) -> Result<String> {
        Ok(self.current().await?.email.unwrap_or_default())
    }

    async fn enable_service(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        service: &str,
    ) -> Result<()> {
        self.set_service_enabled(&emitter, service, true).await
    }

    async fn disable_service(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        service: &str,
    ) -> Result<()> {
        self.set_service_enabled(&emitter, service, false).await
    }

    async fn remove(&self) -> Result<()> {
        let id = self.id;

        {
            let mut config = self.config.lock().await;
            config
                .remove_account(&id)
                .map_err(|e| zbus::fdo::Error::Failed(format!("Account {id} not removed: {e}")))?;
        }

        {
            let auth_manager = self.auth_manager.lock().await;
            auth_manager
                .delete_credentials(&id)
                .await
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        }

        if let Some(connection) = crate::daemon::CONNECTION.get() {
            let path = format!(
                "/dev/edfloreshz/Accounts/Accounts/{}",
                id.to_string().replace('-', "_")
            );
            connection
                .object_server()
                .remove::<AccountInterface, _>(path)
                .await?;

            if let Ok(iface_ref) = connection
                .object_server()
                .interface::<_, crate::daemon::manager::ManagerInterface>(
                    "/dev/edfloreshz/Accounts/Manager",
                )
                .await
            {
                let account_path = zbus::zvariant::OwnedObjectPath::try_from(format!(
                    "/dev/edfloreshz/Accounts/Accounts/{}",
                    id.to_string().replace('-', "_")
                ))
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
                crate::daemon::manager::ManagerInterface::account_removed(
                    iface_ref.signal_emitter(),
                    account_path,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn get_access_token(&self) -> Result<String> {
        let account = self.current().await?;
        let auth_manager = self.auth_manager.lock().await;
        auth_manager
            .get_account_credentials(&account.id)
            .await
            .map(|credentials| credentials.access_token)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn ensure_credentials(&self) -> Result<()> {
        let mut account = self.current().await?;
        let mut auth_manager = self.auth_manager.lock().await;
        auth_manager
            .ensure_credentials(&mut account)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn reauthenticate(&self) -> Result<OwnedObjectPath> {
        let account = self.current().await?;
        create_request(account.provider, Some(self.id), self.auth_manager.clone()).await
    }

    #[zbus(signal)]
    async fn services_changed(
        emitter: &SignalEmitter<'_>,
        enabled_services: Vec<String>,
    ) -> zbus::Result<()>;
}

impl AccountInterface {
    async fn set_service_enabled(
        &self,
        emitter: &SignalEmitter<'_>,
        service: &str,
        enabled: bool,
    ) -> Result<()> {
        let mut account = self.current().await?;
        let Some(service) = Service::from_str(service.to_string()) else {
            return Err(Error::InvalidService(service.to_string()).into());
        };
        account.services.insert(service.clone(), enabled);

        {
            let mut config = self.config.lock().await;
            config
                .save_account(&account)
                .map_err(|e| zbus::fdo::Error::Failed(format!("Failed to save account: {e}")))?;
        }

        if let Some(svc) = ServiceFactory::create_service(&account, &service) {
            if enabled {
                svc.add_service().await?;
            } else {
                svc.remove_service().await?;
            }
        }

        let enabled_services: Vec<String> = account
            .services
            .iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(service, _)| service.to_string())
            .collect();

        Self::services_changed(emitter, enabled_services)
            .await
            .map_err(Into::into)
    }
}
