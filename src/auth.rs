use crate::{Account, AccountsError, Credentials, Provider, ProviderConfig, Result};
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
use uuid::Uuid;

pub struct AuthManager {
    configs: HashMap<Provider, ProviderConfig>,
    pending_auth: HashMap<String, (Provider, PkceCodeVerifier)>,
}

impl AuthManager {
    pub fn new() -> Self {
        let configs = HashMap::new();

        // Add default provider configurations
        // Note: In a real implementation, these would be loaded from config files
        // and client secrets would be handled more securely

        Self {
            configs,
            pending_auth: HashMap::new(),
        }
    }

    pub fn add_provider_config(&mut self, provider: Provider, config: ProviderConfig) {
        self.configs.insert(provider.clone(), config);
        println!("Provider configuration added: {}", provider.to_string());
    }

    pub async fn start_auth_flow(&mut self, provider: Provider) -> Result<String> {
        let config = self
            .configs
            .get(&provider)
            .ok_or(AccountsError::InvalidProviderConfig)?;

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
        let (provider, pkce_verifier) = self.pending_auth.remove(&csrf_token).ok_or_else(|| {
            AccountsError::AuthenticationFailed {
                reason: "Invalid CSRF token".to_string(),
            }
        })?;

        let config = self
            .configs
            .get(&provider)
            .ok_or(AccountsError::InvalidProviderConfig)?;

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

        let credentials = Credentials {
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
            credentials,
            capabilities: provider.default_capabilities(),
        };

        Ok(account)
    }

    async fn get_user_info(&self, provider: &Provider, access_token: &str) -> Result<UserInfo> {
        let client = reqwest::Client::new();

        let user_info_url = match provider {
            Provider::Google => "https://www.googleapis.com/oauth2/v2/userinfo",
            Provider::Microsoft => "https://graph.microsoft.com/v1.0/me",
            Provider::GitHub => "https://api.github.com/user",
            Provider::GitLab => "https://gitlab.com/api/v4/user",
            _ => {
                return Err(AccountsError::AuthenticationFailed {
                    reason: "Unsupported provider for user info".to_string(),
                })
            }
        };

        let response = client
            .get(user_info_url)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AccountsError::AuthenticationFailed {
                reason: format!("Failed to get user info: {}", response.status()),
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
            Provider::GitHub => UserInfo {
                display_name: user_data["name"]
                    .as_str()
                    .unwrap_or_else(|| user_data["login"].as_str().unwrap_or("Unknown"))
                    .to_string(),
                username: user_data["login"].as_str().unwrap_or("Unknown").to_string(),
                email: user_data["email"].as_str().map(|s| s.to_string()),
            },
            Provider::GitLab => UserInfo {
                display_name: user_data["name"].as_str().unwrap_or("Unknown").to_string(),
                username: user_data["username"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_string(),
                email: user_data["email"].as_str().map(|s| s.to_string()),
            },
            _ => {
                return Err(AccountsError::AuthenticationFailed {
                    reason: "Unsupported provider".to_string(),
                })
            }
        };

        Ok(user_info)
    }

    pub async fn refresh_token(&self, account: &mut Account) -> Result<()> {
        let config = self
            .configs
            .get(&account.provider)
            .ok_or(AccountsError::InvalidProviderConfig)?;

        let refresh_token = account.credentials.refresh_token.as_ref().ok_or_else(|| {
            AccountsError::TokenExpired {
                account_id: account.id.to_string(),
            }
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

        account.credentials.access_token = token_result.access_token().secret().clone();
        if let Some(new_refresh_token) = token_result.refresh_token() {
            account.credentials.refresh_token = Some(new_refresh_token.secret().clone());
        }
        account.credentials.expires_at = token_result
            .expires_in()
            .map(|duration| Utc::now() + Duration::seconds(duration.as_secs() as i64));

        Ok(())
    }
}

struct UserInfo {
    display_name: String,
    username: String,
    email: Option<String>,
}
