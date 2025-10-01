use crate::{Error, auth::AuthManager, services::ServiceFactory};
use accounts::{
    config::AccountsConfig,
    models::{DbusAccount, Provider, Service},
};
use uuid::Uuid;
use zbus::{fdo::Result, interface, object_server::SignalEmitter};

pub struct AccountsInterface {
    auth_manager: AuthManager,
    config: AccountsConfig,
}

#[interface(name = "dev.edfloreshz.Accounts.Account")]
impl AccountsInterface {
    /// List all accounts
    pub(crate) async fn list_accounts(&self) -> Vec<DbusAccount> {
        self.config.accounts.iter().map(Into::into).collect()
    }

    /// Get a specific account by ID
    async fn get_account(&self, id: &str) -> Result<DbusAccount> {
        let uuid = Uuid::parse_str(id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        match self
            .config
            .accounts
            .iter()
            .find(|account| account.id == uuid)
        {
            Some(account) => Ok(account.into()),
            None => Err(Error::AccountNotFound(id.to_string()).into()),
        }
    }

    /// Start OAuth2 authentication flow for a provider
    async fn start_authentication(&mut self, provider_name: &str) -> Result<String> {
        let provider = Provider::from_str(provider_name);

        let Some(provider) = provider else {
            return Err(Error::InvalidProvider(provider_name.to_string()).into());
        };

        match self.auth_manager.start_auth_flow(provider).await {
            Ok(url) => Ok(url),
            Err(err) => {
                tracing::error!("Failed to start authentication flow: {}", err);
                Err(Error::AuthenticationFailed {
                    reason: err.to_string(),
                }
                .into())
            }
        }
    }

    /// Complete OAuth2 authentication flow
    async fn complete_authentication(
        &mut self,
        csrf_token: &str,
        authorization_code: &str,
    ) -> Result<String> {
        match self
            .auth_manager
            .complete_auth_flow(csrf_token.to_string(), authorization_code.to_string())
            .await
        {
            Ok(account) => {
                let account_id = account.id.to_string();
                match self.config.save_account(&account) {
                    Ok(_) => Ok(account_id),
                    Err(err) => Err(Error::AccountNotSaved(err.to_string()).into()),
                }
            }
            Err(err) => Err(Error::AuthenticationFailed {
                reason: err.to_string(),
            }
            .into()),
        }
    }

    /// Remove an account
    async fn remove_account(&mut self, id: &str) -> Result<()> {
        let id = Uuid::parse_str(id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        self.config
            .remove_account(&id)
            .map_err(|e| zbus::fdo::Error::Failed(format!("Account {id} not removed: {}", e)))?;
        self.auth_manager
            .delete_credentials(&id)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }

    /// Enable or disable an account
    async fn set_account_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        let uuid = Uuid::parse_str(id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        match self.config.get_account(&uuid) {
            Some(mut account) => {
                account.enabled = enabled;
                match self.config.save_account(&account) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(Error::AccountNotUpdated(format!(
                        "Account {id} not updated: {}",
                        err
                    ))
                    .into()),
                }
            }
            None => Err(Error::AccountNotFound(id.to_string()).into()),
        }
    }

    async fn set_service_enabled(&mut self, id: &str, service: &str, enabled: bool) -> Result<()> {
        let uuid = Uuid::parse_str(id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let Some(mut account) = self.config.get_account(&uuid) else {
            return Err(Error::AccountNotFound(id.to_string()).into());
        };
        let Some(service) = Service::from_str(service.to_string()) else {
            return Err(Error::InvalidService(service.to_string()).into());
        };
        account.services.insert(service.clone(), enabled);
        self.config
            .save_account(&account)
            .map_err(|e| zbus::fdo::Error::Failed(format!("Failed to save account: {}", e)))?;

        if let Some(service) = ServiceFactory::create_service(&account, &service) {
            if enabled {
                service.add_service().await?;
            } else {
                service.remove_service().await?;
            }
        }
        Ok(())
    }

    async fn ensure_credentials(&mut self) -> Result<()> {
        for account in self.config.accounts.iter_mut() {
            self.auth_manager
                .ensure_credentials(account)
                .await
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        }
        Ok(())
    }

    async fn get_access_token(&mut self, id: &str) -> Result<String> {
        let uuid = Uuid::parse_str(id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        match self.config.get_account(&uuid) {
            Some(account) => self
                .auth_manager
                .get_account_credentials(&account.id)
                .await
                .map(|credentials| credentials.access_token)
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string())),
            None => Err(Error::AccountNotFound(id.to_string()).into()),
        }
    }

    async fn get_refresh_token(&mut self, id: &str) -> Result<String> {
        let uuid = Uuid::parse_str(id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        match self.config.get_account(&uuid) {
            Some(account) => self
                .auth_manager
                .get_account_credentials(&account.id)
                .await
                .map(|credentials| credentials.refresh_token.unwrap_or_default())
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string())),
            None => Err(Error::AccountNotFound(id.to_string()).into()),
        }
    }

    async fn emit_account_added(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        account_id: &str,
    ) -> Result<()> {
        emitter.account_added(account_id).await.map_err(Into::into)
    }

    async fn emit_account_removed(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        account_id: &str,
    ) -> Result<()> {
        emitter
            .account_removed(account_id)
            .await
            .map_err(Into::into)
    }

    async fn emit_account_changed(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        account_id: &str,
    ) -> Result<()> {
        emitter
            .account_changed(account_id)
            .await
            .map_err(Into::into)
    }

    async fn emit_account_exists(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<()> {
        emitter.account_exists().await.map_err(Into::into)
    }

    /// Signals

    #[zbus(signal)]
    async fn account_added(emitter: &SignalEmitter<'_>, account_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn account_removed(emitter: &SignalEmitter<'_>, account_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn account_changed(emitter: &SignalEmitter<'_>, account_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn account_exists(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

impl AccountsInterface {
    pub async fn new() -> crate::Result<Self> {
        Ok(Self {
            auth_manager: AuthManager::new().await?,
            config: AccountsConfig::config(),
        })
    }
}
