use crate::{
    auth::AuthManager, storage::AccountStorage, zbus::Account, AccountsError, Provider,
    ProviderConfig, Result,
};
use chrono::Utc;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use uuid::Uuid;
use zbus::{interface, SignalContext};

#[derive(Deserialize)]
struct AccountProviderConfig {
    provider: AccountProvider,
}

#[derive(Deserialize)]
struct AccountProvider {
    client_id: String,
    client_secret: String,
    auth_url: String,
    token_url: String,
    redirect_uri: String,
    scopes: Vec<String>,
}

pub struct CosmicAccounts {
    storage: AccountStorage,
    auth_manager: AuthManager,
}

impl CosmicAccounts {
    pub fn new() -> Result<Self> {
        Ok(Self {
            storage: AccountStorage::new()?,
            auth_manager: AuthManager::new(),
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
            ("github.toml", Provider::GitHub),
            ("gitlab.toml", Provider::GitLab),
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
                            provider.display_name(),
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
    ) -> Result<ProviderConfig> {
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

#[interface(name = "com.system76.CosmicAccounts")]
impl CosmicAccounts {
    /// List all accounts
    async fn list_accounts(&self) -> Vec<Account> {
        match self.storage.list_accounts() {
            Ok(accounts) => accounts.iter().map(|account| account.into()).collect(),
            Err(err) => {
                tracing::error!("Failed to list accounts: {}", err);
                vec![]
            }
        }
    }

    /// Get a specific account by ID
    async fn get_account(&self, id: &str) -> zbus::fdo::Result<Account> {
        let uuid = Uuid::parse_str(id).unwrap();
        match self.storage.get_account(&uuid) {
            Ok(Some(account)) => Ok(account.into()),
            Ok(None) => Err(AccountsError::AccountNotFound(id.to_string()).into()),
            Err(err) => Err(AccountsError::StorageError(err.to_string()).into()),
        }
    }

    /// Start OAuth2 authentication flow for a provider
    async fn start_authentication(&mut self, provider_name: &str) -> zbus::fdo::Result<String> {
        let provider = Provider::from_str(provider_name);

        let Some(provider) = provider else {
            return Err(AccountsError::InvalidProvider(provider_name.to_string()).into());
        };

        match self.auth_manager.start_auth_flow(provider).await {
            Ok(url) => Ok(url),
            Err(err) => {
                tracing::error!("Failed to start authentication flow: {}", err);
                Err(AccountsError::AuthenticationFailed {
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
    ) -> zbus::fdo::Result<String> {
        match self
            .auth_manager
            .complete_auth_flow(csrf_token.to_string(), authorization_code.to_string())
            .await
        {
            Ok(account) => {
                let account_id = account.id.to_string();
                match self.storage.save_account(&account) {
                    Ok(_) => {
                        // Note: Signal emission would be handled by the D-Bus framework
                        Ok(account_id)
                    }
                    Err(err) => Err(AccountsError::AccountNotSaved(err.to_string()).into()),
                }
            }
            Err(err) => Err(AccountsError::AuthenticationFailed {
                reason: err.to_string(),
            }
            .into()),
        }
    }

    /// Remove an account
    async fn remove_account(&mut self, id: &str) -> zbus::fdo::Result<()> {
        let uuid = Uuid::parse_str(id).unwrap();

        match self.storage.remove_account(&uuid) {
            Ok(_) => {
                // Note: Signal emission would be handled by the D-Bus framework
                Ok(())
            }
            Err(err) => Err(AccountsError::AccountNotRemoved(format!(
                "Account {id} not removed: {}",
                err
            ))
            .into()),
        }
    }

    /// Enable or disable an account
    async fn set_account_enabled(&mut self, id: &str, enabled: bool) -> zbus::fdo::Result<()> {
        let uuid = Uuid::parse_str(id).unwrap();

        match self.storage.get_account(&uuid) {
            Ok(Some(mut account)) => {
                account.enabled = enabled;
                match self.storage.save_account(&account) {
                    Ok(_) => {
                        // Note: Signal emission would be handled by the D-Bus framework
                        Ok(())
                    }
                    Err(err) => Err(AccountsError::AccountNotUpdated(format!(
                        "Account {id} not updated: {}",
                        err
                    ))
                    .into()),
                }
            }
            Ok(None) => Err(AccountsError::AccountNotFound(id.to_string()).into()),
            Err(err) => Err(AccountsError::StorageError(err.to_string()).into()),
        }
    }

    /// Get access token for an account (refreshing if necessary)
    async fn get_access_token(&mut self, id: &str) -> zbus::fdo::Result<String> {
        let uuid = Uuid::parse_str(id).unwrap();

        match self.storage.get_account(&uuid) {
            Ok(Some(mut account)) => {
                // Check if token is expired and refresh if necessary
                if let Some(expires_at) = account.credentials.expires_at {
                    if expires_at <= Utc::now() {
                        match self.auth_manager.refresh_token(&mut account).await {
                            Ok(_) => {
                                self.storage.save_account(&account).ok();
                            }
                            Err(_) => {
                                return Err(
                                    AccountsError::TokenRefreshFailed(id.to_string()).into()
                                );
                            }
                        }
                    }
                }

                Ok(account.credentials.access_token)
            }
            Ok(None) => Err(AccountsError::AccountNotFound(id.to_string()).into()),
            Err(err) => Err(AccountsError::StorageError(err.to_string()).into()),
        }
    }

    /// Signals
    #[zbus(signal)]
    async fn account_added(ctxt: &SignalContext<'_>, account_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn account_removed(ctxt: &SignalContext<'_>, account_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn account_changed(ctxt: &SignalContext<'_>, account_id: &str) -> zbus::Result<()>;
}

// AccountInfo is now represented as HashMap<String, String> for D-Bus compatibility
