use crate::{auth::AuthManager, storage::AccountStorage, Provider, ProviderConfig, Result};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
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
        let storage = AccountStorage::new()?;
        let auth_manager = AuthManager::new();

        // Initialize with default provider configurations
        // In a real implementation, these would be loaded from config files

        Ok(Self {
            storage,
            auth_manager,
        })
    }

    pub async fn setup_providers(&mut self) -> Result<()> {
        // Load provider configurations from TOML files in data/providers/
        let providers_dir = Path::new("data/providers");

        if !providers_dir.exists() {
            return Ok(()); // No providers directory, skip setup
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
    /// List all accounts (returns array of dictionaries)
    async fn list_accounts(&self) -> Vec<HashMap<String, String>> {
        match self.storage.list_accounts() {
            Ok(accounts) => accounts
                .into_iter()
                .map(|account| {
                    let mut info = HashMap::new();
                    info.insert("id".to_string(), account.id.to_string());
                    info.insert(
                        "provider".to_string(),
                        account.provider.display_name().to_string(),
                    );
                    info.insert("display_name".to_string(), account.display_name);
                    info.insert("username".to_string(), account.username);
                    info.insert("email".to_string(), account.email.unwrap_or_default());
                    info.insert("enabled".to_string(), account.enabled.to_string());
                    info.insert(
                        "capabilities".to_string(),
                        account
                            .capabilities
                            .iter()
                            .map(|c| format!("{:?}", c))
                            .collect::<Vec<_>>()
                            .join(","),
                    );
                    info
                })
                .collect(),
            Err(_) => vec![],
        }
    }

    /// Get a specific account by ID (returns dictionary)
    async fn get_account(
        &self,
        id: &str,
    ) -> std::result::Result<HashMap<String, String>, zbus::fdo::Error> {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| zbus::fdo::Error::InvalidArgs("Invalid account ID".to_string()))?;

        match self.storage.get_account(&uuid) {
            Ok(Some(account)) => {
                let mut info = HashMap::new();
                info.insert("id".to_string(), account.id.to_string());
                info.insert(
                    "provider".to_string(),
                    account.provider.display_name().to_string(),
                );
                info.insert("display_name".to_string(), account.display_name);
                info.insert("username".to_string(), account.username);
                info.insert("email".to_string(), account.email.unwrap_or_default());
                info.insert("enabled".to_string(), account.enabled.to_string());
                info.insert(
                    "capabilities".to_string(),
                    account
                        .capabilities
                        .iter()
                        .map(|c| format!("{:?}", c))
                        .collect::<Vec<_>>()
                        .join(","),
                );
                Ok(info)
            }
            Ok(None) => Err(zbus::fdo::Error::Failed("Account not found".to_string())),
            Err(_) => Err(zbus::fdo::Error::Failed("Storage error".to_string())),
        }
    }

    /// Start OAuth2 authentication flow for a provider
    async fn start_authentication(
        &mut self,
        provider_name: &str,
    ) -> std::result::Result<String, zbus::fdo::Error> {
        let provider = match provider_name {
            "Google" => Provider::Google,
            "Microsoft" => Provider::Microsoft,
            "GitHub" => Provider::GitHub,
            "GitLab" => Provider::GitLab,
            _ => {
                return Err(zbus::fdo::Error::InvalidArgs(
                    "Unsupported provider".to_string(),
                ))
            }
        };

        match self.auth_manager.start_auth_flow(provider).await {
            Ok(auth_url) => {
                open::that(&auth_url).unwrap();
                Ok(auth_url)
            }
            Err(e) => Err(zbus::fdo::Error::Failed(format!(
                "Authentication failed: {}",
                e
            ))),
        }
    }

    /// Complete OAuth2 authentication flow
    async fn complete_authentication(
        &mut self,
        csrf_token: &str,
        authorization_code: &str,
    ) -> std::result::Result<String, zbus::fdo::Error> {
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
                    Err(_) => Err(zbus::fdo::Error::Failed(
                        "Failed to save account".to_string(),
                    )),
                }
            }
            Err(e) => Err(zbus::fdo::Error::Failed(format!(
                "Authentication failed: {}",
                e
            ))),
        }
    }

    /// Remove an account
    async fn remove_account(&mut self, id: &str) -> std::result::Result<(), zbus::fdo::Error> {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| zbus::fdo::Error::InvalidArgs("Invalid account ID".to_string()))?;

        match self.storage.remove_account(&uuid) {
            Ok(_) => {
                // Note: Signal emission would be handled by the D-Bus framework
                Ok(())
            }
            Err(_) => Err(zbus::fdo::Error::Failed(
                "Failed to remove account".to_string(),
            )),
        }
    }

    /// Enable or disable an account
    async fn set_account_enabled(
        &mut self,
        id: &str,
        enabled: bool,
    ) -> std::result::Result<(), zbus::fdo::Error> {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| zbus::fdo::Error::InvalidArgs("Invalid account ID".to_string()))?;

        match self.storage.get_account(&uuid) {
            Ok(Some(mut account)) => {
                account.enabled = enabled;
                match self.storage.save_account(&account) {
                    Ok(_) => {
                        // Note: Signal emission would be handled by the D-Bus framework
                        Ok(())
                    }
                    Err(_) => Err(zbus::fdo::Error::Failed(
                        "Failed to update account".to_string(),
                    )),
                }
            }
            Ok(None) => Err(zbus::fdo::Error::Failed("Account not found".to_string())),
            Err(_) => Err(zbus::fdo::Error::Failed("Storage error".to_string())),
        }
    }

    /// Get access token for an account (refreshing if necessary)
    async fn get_access_token(
        &mut self,
        id: &str,
    ) -> std::result::Result<String, zbus::fdo::Error> {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| zbus::fdo::Error::InvalidArgs("Invalid account ID".to_string()))?;

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
                                return Err(zbus::fdo::Error::Failed(
                                    "Failed to refresh token".to_string(),
                                ));
                            }
                        }
                    }
                }

                Ok(account.credentials.access_token)
            }
            Ok(None) => Err(zbus::fdo::Error::Failed("Account not found".to_string())),
            Err(_) => Err(zbus::fdo::Error::Failed("Storage error".to_string())),
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
