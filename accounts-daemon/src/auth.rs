use accounts::{
    config::AccountsConfig,
    models::{Account, Credential, Provider},
};
use chrono::{Duration, Utc};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::models::AccountProviderConfig;
use crate::{error::*, models::ProviderConfig, storage::CredentialStorage};

pub struct AuthManager {
    configs: HashMap<Provider, ProviderConfig>,
    pending_auth: HashMap<String, (Provider, PkceCodeVerifier)>,
    storage: CredentialStorage,
    config: AccountsConfig,
}

impl AuthManager {
    pub async fn new() -> Result<Self> {
        let mut configs = HashMap::new();

        for provider in Provider::list() {
            if let Some(mut config) = Self::load_provider_config(&provider)? {
                Self::apply_env_overrides(&provider, &mut config);
                configs.insert(provider, config);
            } else {
                tracing::error!("Provider config for {:?} not found in any location", provider);
            }
        }

        Ok(Self {
            configs,
            pending_auth: HashMap::new(),
            storage: CredentialStorage::new().await?,
            config: AccountsConfig::config(),
        })
    }

    fn load_provider_config(provider: &Provider) -> Result<Option<ProviderConfig>> {
        let file_name = provider.file_name();
        let paths = vec![
            // 1. User config: ~/.config/cosmic/accounts/providers/
            std::env::var("XDG_CONFIG_HOME")
                .map(|p| Path::new(&p).to_path_buf())
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_default();
                    Path::new(&home).join(".config")
                })
                .join("cosmic/accounts/providers")
                .join(file_name),
            // 2. System config: /etc/cosmic/accounts/providers/
            Path::new("/etc/cosmic/accounts/providers").join(file_name),
            // 3. System data: /usr/share/cosmic/accounts/providers/
            Path::new("/usr/share/cosmic/accounts/providers").join(file_name),
            // 4. Built-in/Dev path
            Path::new("accounts-daemon/data/providers").join(file_name),
        ];

        for path in paths {
            if path.exists() {
                tracing::info!("Loading {:?} config from: {}", provider, path.display());
                let content = std::fs::read_to_string(path)?;
                let toml_config: AccountProviderConfig = toml::from_str(&content)?;
                return Ok(Some(toml_config.provider));
            }
        }

        Ok(None)
    }

    fn apply_env_overrides(provider: &Provider, config: &mut ProviderConfig) {
        let provider_env = match provider {
            Provider::Google => "GOOGLE",
            Provider::Microsoft => "MICROSOFT",
        };

        let client_id_env = format!("COSMIC_ACCOUNTS_{}_CLIENT_ID", provider_env);
        let client_secret_env = format!("COSMIC_ACCOUNTS_{}_CLIENT_SECRET", provider_env);

        if let Ok(val) = std::env::var(&client_id_env) {
            tracing::info!("Overriding {:?} client_id from env: {}", provider, client_id_env);
            config.client_id = val;
        }

        if let Ok(val) = std::env::var(&client_secret_env) {
            tracing::info!("Overriding {:?} client_secret from env: {}", provider, client_secret_env);
            config.client_secret = val;
        }
    }

    pub async fn start_auth_flow(&mut self, provider: Provider) -> Result<String> {
        let config = self
            .configs
            .get(&provider)
            .ok_or(Error::InvalidProviderConfig)?;

        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            AuthUrl::new(config.auth_url.clone())?,
            Some(TokenUrl::new(config.token_url.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut auth_request = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);

        for scope in &config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        // Add access_type=offline for Google to get refresh tokens
        if matches!(provider, Provider::Google) {
            auth_request = auth_request.add_extra_param("access_type", "offline");
        }

        let (auth_url, csrf_token) = auth_request.url();

        // Store the PKCE verifier for later use
        self.pending_auth
            .insert(csrf_token.secret().clone(), (provider, pkce_verifier));

        Ok(auth_url.to_string())
    }

    pub async fn complete_auth_flow(
        &mut self,
        csrf_token: String,
        authorization_code: String,
    ) -> Result<Account> {
        let (provider, pkce_verifier) =
            self.pending_auth
                .remove(&csrf_token)
                .ok_or_else(|| Error::AuthenticationFailed {
                    reason: "Invalid CSRF token".to_string(),
                })?;

        let config = self
            .configs
            .get(&provider)
            .ok_or(Error::InvalidProviderConfig)?;

        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            AuthUrl::new(config.auth_url.clone())?,
            Some(TokenUrl::new(config.token_url.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

        let token_result = client
            .exchange_code(AuthorizationCode::new(authorization_code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await?;

        let access_token = token_result.access_token().secret();
        let refresh_token = token_result.refresh_token().map(|t| t.secret().clone());
        let expires_at = token_result
            .expires_in()
            .map(|duration| Utc::now() + Duration::seconds(duration.as_secs() as i64));

        // Get user information
        let user_info = self.get_user_info(&provider, access_token).await?;

        if self.config.account_exists(&user_info.username, &provider) {
            return Err(Error::AccountAlreadyExists);
        }

        let credentials = Credential {
            access_token: access_token.clone(),
            refresh_token,
            expires_at,
            scope: config.scopes.clone(),
            token_type: "Bearer".to_string(),
        };

        let account = Account {
            id: Uuid::new_v4(),
            provider: provider.clone(),
            display_name: user_info.display_name,
            username: user_info.username,
            email: user_info.email,
            enabled: true,
            created_at: Utc::now(),
            last_used: Some(Utc::now()),
            services: provider.services(),
        };

        self.storage
            .set_account_credentials(&account.id, &credentials)
            .await?;

        Ok(account)
    }

    async fn get_user_info(&self, provider: &Provider, access_token: &str) -> Result<UserInfo> {
        let client = reqwest::Client::new();

        let user_info_url = match provider {
            Provider::Google => "https://www.googleapis.com/oauth2/v2/userinfo",
            Provider::Microsoft => "https://graph.microsoft.com/v1.0/me",
        };

        let response = client
            .get(user_info_url)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or("No error body".to_string());
            tracing::error!("Error response: {}", error_body);
            return Err(Error::AuthenticationFailed {
                reason: format!("Failed to get user info: {} - {}", status, error_body),
            });
        }

        let user_data: Value = response.json().await?;

        let user_info = match provider {
            Provider::Google => UserInfo {
                display_name: user_data["name"].as_str().unwrap_or("Unknown").to_string(),
                username: user_data["email"].as_str().unwrap_or("Unknown").to_string(),
                email: user_data["email"].as_str().map(|s| s.to_string()),
            },
            Provider::Microsoft => UserInfo {
                display_name: user_data["displayName"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_string(),
                username: user_data["userPrincipalName"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_string(),
                email: user_data["mail"]
                    .as_str()
                    .or_else(|| user_data["userPrincipalName"].as_str())
                    .map(|s| s.to_string()),
            },
        };

        Ok(user_info)
    }

    pub async fn refresh_token(&self, account: &Account) -> Result<()> {
        let config = self
            .configs
            .get(&account.provider)
            .ok_or(Error::InvalidProviderConfig)?;

        let mut credentials = self.storage.get_account_credentials(&account.id).await?;

        let refresh_token =
            credentials
                .refresh_token
                .as_ref()
                .ok_or_else(|| Error::TokenExpired {
                    account_id: account.id.to_string(),
                })?;

        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            AuthUrl::new(config.auth_url.clone())?,
            Some(TokenUrl::new(config.token_url.clone())?),
        );

        let token_result = client
            .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token.clone()))
            .request_async(async_http_client)
            .await?;

        credentials.access_token = token_result.access_token().secret().clone();
        if let Some(new_refresh_token) = token_result.refresh_token() {
            credentials.refresh_token = Some(new_refresh_token.secret().clone());
        }
        credentials.expires_at = token_result
            .expires_in()
            .map(|duration| Utc::now() + Duration::seconds(duration.as_secs() as i64));

        self.storage
            .set_account_credentials(&account.id, &credentials)
            .await?;

        Ok(())
    }

    pub async fn ensure_credentials(&mut self, account: &mut Account) -> Result<()> {
        // Check if token is expired and refresh if necessary
        let credentials = self
            .storage
            .get_account_credentials(&account.id)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        if let Some(expires_at) = credentials.expires_at {
            if expires_at <= Utc::now() {
                self.refresh_token(&account).await?;
            }
        }
        Ok(())
    }

    pub async fn delete_credentials(&self, id: &Uuid) -> Result<()> {
        self.storage.delete_account_credentials(id).await?;
        Ok(())
    }

    pub async fn get_account_credentials(&self, id: &Uuid) -> Result<Credential> {
        self.storage.get_account_credentials(id).await
    }
}

struct UserInfo {
    display_name: String,
    username: String,
    email: Option<String>,
}
