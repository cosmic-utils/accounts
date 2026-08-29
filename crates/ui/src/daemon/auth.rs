use accounts_core::{
    config::AccountsConfig, models::Credential, proxy::Provider1Proxy, registry::ProviderRegistry,
};
use chrono::{Duration, Utc};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::collections::HashMap;
use uuid::Uuid;
use zbus::Connection;

use crate::daemon::{error::*, storage::CredentialStorage};

struct PendingAuth {
    provider_id: String,
    verifier: PkceCodeVerifier,
    request_id: String,
    /// Set when this flow refreshes an existing account instead of creating a new one.
    existing_account: Option<Uuid>,
}

pub struct CompletedAuth {
    pub request_id: String,
    pub existing_account: Option<Uuid>,
    pub account: accounts_core::models::Account,
}

pub struct AuthManager {
    pending_auth: HashMap<String, PendingAuth>,
    storage: CredentialStorage,
    connection: Connection,
}

impl AuthManager {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            pending_auth: HashMap::new(),
            storage: CredentialStorage::new().await?,
            connection: Connection::session().await?,
        })
    }

    fn registry() -> Result<&'static ProviderRegistry> {
        crate::daemon::REGISTRY
            .get()
            .ok_or(Error::InvalidProviderConfig)
    }

    fn oauth_client(manifest: &accounts_core::ProviderManifest) -> Result<BasicClient> {
        let oauth = &manifest.oauth;
        Ok(BasicClient::new(
            ClientId::new(oauth.client_id.clone()),
            oauth.client_secret.clone().map(ClientSecret::new),
            AuthUrl::new(oauth.auth_url.clone())?,
            Some(TokenUrl::new(oauth.token_url.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(oauth.redirect_uri.clone())?))
    }

    pub async fn start_auth_flow(
        &mut self,
        provider_id: String,
        request_id: String,
        existing_account: Option<Uuid>,
    ) -> Result<(String, String)> {
        let registry = Self::registry()?;
        let manifest = registry
            .get(&provider_id)
            .ok_or_else(|| Error::InvalidProvider(provider_id.clone()))?;

        let client = Self::oauth_client(manifest)?;

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut auth_request = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);

        for scope in &manifest.oauth.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        for (key, value) in &manifest.oauth.extra_params {
            auth_request = auth_request.add_extra_param(key.clone(), value.clone());
        }

        let (auth_url, csrf_token) = auth_request.url();
        let csrf_secret = csrf_token.secret().clone();

        self.pending_auth.insert(
            csrf_secret.clone(),
            PendingAuth {
                provider_id,
                verifier: pkce_verifier,
                request_id,
                existing_account,
            },
        );

        Ok((auth_url.to_string(), csrf_secret))
    }

    /// Removes a pending flow without completing it, e.g. on `Request.Cancel` or timeout.
    pub fn discard_pending(&mut self, csrf_token: &str) {
        self.pending_auth.remove(csrf_token);
    }

    /// Looks up the `Request` id for a pending flow without consuming it, used when the
    /// OAuth redirect carries a provider-side error instead of a code.
    pub fn request_id_for_csrf(&self, csrf_token: &str) -> Option<String> {
        self.pending_auth
            .get(csrf_token)
            .map(|pending| pending.request_id.clone())
    }

    pub async fn complete_auth_flow(
        &mut self,
        csrf_token: String,
        authorization_code: String,
    ) -> Result<CompletedAuth> {
        let pending = self
            .pending_auth
            .remove(&csrf_token)
            .ok_or_else(|| Error::AuthenticationFailed {
                reason: "Invalid CSRF token".to_string(),
            })?;

        let registry = Self::registry()?;
        let manifest = registry
            .get(&pending.provider_id)
            .ok_or_else(|| Error::InvalidProvider(pending.provider_id.clone()))?;

        let client = Self::oauth_client(manifest)?;

        let token_result = client
            .exchange_code(AuthorizationCode::new(authorization_code))
            .set_pkce_verifier(pending.verifier)
            .request_async(async_http_client)
            .await?;

        let access_token = token_result.access_token().secret();
        let refresh_token = token_result.refresh_token().map(|t| t.secret().clone());
        let expires_at = token_result
            .expires_in()
            .map(|duration| Utc::now() + Duration::seconds(duration.as_secs() as i64));

        let user_info = self.get_user_info(manifest, access_token).await?;

        let credentials = Credential {
            access_token: access_token.clone(),
            refresh_token,
            expires_at,
            scope: manifest.oauth.scopes.clone(),
            token_type: "Bearer".to_string(),
        };

        let account = if let Some(existing_id) = pending.existing_account {
            let mut account = AccountsConfig::config()
                .get_account(&existing_id)
                .ok_or_else(|| Error::AccountNotFound(existing_id.to_string()))?;
            account.last_used = Some(Utc::now());

            self.storage
                .set_account_credentials(&account.id, &credentials)
                .await?;

            account
        } else {
            if AccountsConfig::config().account_exists(&user_info.username, &pending.provider_id) {
                return Err(Error::AccountAlreadyExists);
            }

            let account = accounts_core::models::Account {
                id: Uuid::new_v4(),
                provider: pending.provider_id,
                display_name: user_info.display_name,
                username: user_info.username,
                email: user_info.email,
                enabled: true,
                created_at: Utc::now(),
                last_used: Some(Utc::now()),
                services: manifest.default_services(),
            };

            self.storage
                .set_account_credentials(&account.id, &credentials)
                .await?;

            account
        };

        Ok(CompletedAuth {
            request_id: pending.request_id,
            existing_account: pending.existing_account,
            account,
        })
    }

    async fn get_user_info(
        &self,
        manifest: &accounts_core::ProviderManifest,
        access_token: &str,
    ) -> Result<UserInfo> {
        let proxy = Provider1Proxy::new(&self.connection, manifest.provider.dbus_name.clone())
            .await
            .map_err(Error::DBus)?;

        let mut info =
            proxy
                .get_user_info(access_token)
                .await
                .map_err(|e| Error::AuthenticationFailed {
                    reason: format!(
                        "Provider {} failed to return user info: {e}",
                        manifest.provider.id
                    ),
                })?;

        Ok(UserInfo {
            display_name: info
                .remove("display_name")
                .unwrap_or_else(|| "Unknown".to_string()),
            username: info
                .remove("username")
                .unwrap_or_else(|| "Unknown".to_string()),
            email: info.remove("email"),
        })
    }

    pub async fn refresh_token(&self, account: &accounts_core::models::Account) -> Result<()> {
        let registry = Self::registry()?;
        let manifest = registry
            .get(&account.provider)
            .ok_or_else(|| Error::InvalidProvider(account.provider.clone()))?;

        let mut credentials = self.storage.get_account_credentials(&account.id).await?;

        let refresh_token =
            credentials
                .refresh_token
                .as_ref()
                .ok_or_else(|| Error::TokenExpired {
                    account_id: account.id.to_string(),
                })?;

        let client = Self::oauth_client(manifest)?;

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

    pub async fn ensure_credentials(
        &mut self,
        account: &mut accounts_core::models::Account,
    ) -> Result<()> {
        let credentials = self
            .storage
            .get_account_credentials(&account.id)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        if let Some(expires_at) = credentials.expires_at {
            if expires_at <= Utc::now() {
                self.refresh_token(account).await?;
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
