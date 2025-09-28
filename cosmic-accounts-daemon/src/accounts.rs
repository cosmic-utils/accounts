use crate::{
    auth::AuthManager,
    models::{AccountProviderConfig, ProviderConfig},
    storage::CredentialStorage,
    Error,
};
use cosmic_accounts::{
    models::{DbusAccount, Provider},
    CosmicAccountsConfig,
};
use std::fs;
use std::path::Path;
use uuid::Uuid;
use zbus::{fdo::Result, interface, object_server::SignalEmitter};

pub struct CosmicAccounts {
    storage: CredentialStorage,
    config: CosmicAccountsConfig,
    auth_manager: AuthManager,
}

#[interface(name = "com.system76.CosmicAccounts")]
impl CosmicAccounts {
    /// List all accounts
    async fn list_accounts(&self) -> Vec<DbusAccount> {
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
        let uuid = Uuid::parse_str(id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        match self.config.remove_account(&uuid) {
            Ok(_) => {
                // Note: Signal emission would be handled by the D-Bus framework
                Ok(())
            }
            Err(err) => {
                Err(Error::AccountNotRemoved(format!("Account {id} not removed: {}", err)).into())
            }
        }
    }

    /// Enable or disable an account
    async fn set_account_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        let uuid = Uuid::parse_str(id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        match self.config.get_account(&uuid) {
            Some(mut account) => {
                account.enabled = enabled;
                match self.config.save_account(&account) {
                    Ok(_) => {
                        // Note: Signal emission would be handled by the D-Bus framework
                        Ok(())
                    }
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

    async fn ensure_credentials(&mut self) -> Result<()> {
        for account in self.config.accounts.iter_mut() {
            self.auth_manager
                .ensure_credentials(account)
                .await
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        }
        Ok(())
    }

    /// Get access token for an account (refreshing if necessary)
    async fn get_access_token(&mut self, id: &str) -> Result<String> {
        let uuid = Uuid::parse_str(id).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        match self.config.get_account(&uuid) {
            Some(account) => {
                let credentials = self
                    .storage
                    .get_account_credentials(&account.id)
                    .await
                    .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

                Ok(credentials.access_token)
            }
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

    /// Signals

    #[zbus(signal)]
    async fn account_added(emitter: &SignalEmitter<'_>, account_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn account_removed(emitter: &SignalEmitter<'_>, account_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn account_changed(emitter: &SignalEmitter<'_>, account_id: &str) -> zbus::Result<()>;
}

impl CosmicAccounts {
    pub async fn new() -> crate::Result<Self> {
        Ok(Self {
            storage: CredentialStorage::new().await?,
            auth_manager: AuthManager::new().await?,
            config: CosmicAccountsConfig::config(),
        })
    }

    pub async fn setup_providers(&mut self) -> Result<()> {
        let providers_dir = Path::new("data/providers");

        if !providers_dir.exists() {
            return Ok(());
        }

        let provider_files = [
            ("google.toml", Provider::Google),
            ("microsoft.toml", Provider::Microsoft),
        ];

        for (filename, provider) in provider_files.iter() {
            let config_path = providers_dir.join(filename);

            if config_path.exists() {
                match self.load_provider_config(&config_path, provider.clone()) {
                    Ok(config) => {
                        self.auth_manager
                            .add_provider_config(provider.clone(), config);
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to load provider config for {}: {}",
                            provider.to_string(),
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn load_provider_config(
        &self,
        config_path: &Path,
        _provider: Provider,
    ) -> crate::Result<ProviderConfig> {
        let content = fs::read_to_string(config_path)?;
        let toml_config: AccountProviderConfig = toml::from_str(&content)?;

        Ok(ProviderConfig {
            client_id: toml_config.provider.client_id,
            client_secret: toml_config.provider.client_secret,
            auth_url: toml_config.provider.auth_url,
            token_url: toml_config.provider.token_url,
            redirect_uri: toml_config.provider.redirect_uri,
            scopes: toml_config.provider.scopes,
        })
    }
}
