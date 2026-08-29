use accounts_core::{
    config::AccountsConfig, models::Credential, proxy::HANDLER_BLOB_PARAM,
    registry::ProviderRegistry,
};
use base64::Engine;
use chrono::{Duration, Utc};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::daemon::{error::*, storage::CredentialStorage};

/// A `Credential` wrapping an opaque `ProviderHandler` blob.
fn handler_credential(blob: Vec<u8>) -> Credential {
    Credential {
        access_token: String::new(),
        refresh_token: None,
        expires_at: None,
        scope: Vec::new(),
        token_type: "handler".to_string(),
        credential_blob: Some(blob),
    }
}

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
}

impl AuthManager {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            pending_auth: HashMap::new(),
            storage: CredentialStorage::new().await?,
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
        params: HashMap<String, String>,
    ) -> Result<(String, String)> {
        let registry = Self::registry()?;
        let manifest = registry
            .get(&provider_id)
            .ok_or_else(|| Error::InvalidProvider(provider_id.clone()))?;

        if manifest.handler.is_some() {
            return Err(Error::AuthenticationFailed {
                reason: format!(
                    "provider {provider_id} declares a [handler]; use the handler flow, not the OAuth2 flow"
                ),
            });
        }

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

        // Caller hints from `Manager.CreateAccount` params (e.g. `login_hint`)
        // go on the authorize URL, after the manifest's own extra params.
        for (key, value) in &params {
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
        let pending =
            self.pending_auth
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
            credential_blob: None,
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

    /// Auth path for providers that declare a `[handler]`: the daemon can't
    /// drive their flow itself, so it calls the external
    /// `dev.edfloreshz.Accounts.ProviderHandler` service, which returns an
    /// identity and an opaque credential blob the daemon stores verbatim.
    pub async fn authenticate_via_handler(
        &mut self,
        provider_id: String,
        request_id: String,
        existing_account: Option<Uuid>,
        params: HashMap<String, String>,
    ) -> Result<CompletedAuth> {
        let registry = Self::registry()?;
        let manifest = registry
            .get(&provider_id)
            .ok_or_else(|| Error::InvalidProvider(provider_id.clone()))?;

        let (identity, blob) = Self::call_handler(manifest, params).await?;

        let credentials = handler_credential(blob);

        let account = if let Some(existing_id) = existing_account {
            let mut account = AccountsConfig::config()
                .get_account(&existing_id)
                .ok_or_else(|| Error::AccountNotFound(existing_id.to_string()))?;
            account.last_used = Some(Utc::now());
            self.storage
                .set_account_credentials(&account.id, &credentials)
                .await?;
            account
        } else {
            if AccountsConfig::config().account_exists(&identity, &provider_id) {
                return Err(Error::AccountAlreadyExists);
            }
            let email = identity.contains('@').then(|| identity.clone());
            let account = accounts_core::models::Account {
                id: Uuid::new_v4(),
                provider: provider_id,
                display_name: identity.clone(),
                username: identity,
                email,
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
            request_id,
            existing_account,
            account,
        })
    }

    /// Re-runs `ProviderHandler.Authenticate` for an existing handler-based
    /// account, passing the stored blob back so the handler can mint a fresh
    /// credential (its own "later invocation" — the interface stays one method
    /// wide). Called when `Credentials.InvalidateToken` marked the account stale.
    pub async fn refresh_handler_credentials(&mut self, account_id: &Uuid) -> Result<Credential> {
        let account = AccountsConfig::config()
            .get_account(account_id)
            .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;
        let registry = Self::registry()?;
        let manifest = registry
            .get(&account.provider)
            .ok_or_else(|| Error::InvalidProvider(account.provider.clone()))?;

        let mut params = HashMap::new();
        if let Some(blob) = self
            .storage
            .get_account_credentials(account_id)
            .await
            .ok()
            .and_then(|c| c.credential_blob)
        {
            params.insert(
                HANDLER_BLOB_PARAM.to_string(),
                base64::engine::general_purpose::STANDARD.encode(blob),
            );
        }

        let (_identity, blob) = Self::call_handler(manifest, params).await?;
        let credentials = handler_credential(blob);
        self.storage
            .set_account_credentials(account_id, &credentials)
            .await?;
        Ok(credentials)
    }

    async fn call_handler(
        manifest: &accounts_core::ProviderManifest,
        params: HashMap<String, String>,
    ) -> Result<(String, Vec<u8>)> {
        let handler = manifest
            .handler
            .as_ref()
            .ok_or_else(|| Error::AuthenticationFailed {
                reason: format!("provider {} has no [handler] section", manifest.provider.id),
            })?;

        let connection = zbus::Connection::session().await?;
        let proxy = accounts_core::proxy::ProviderHandlerProxy::builder(&connection)
            .destination(handler.bus_name.clone())
            .map_err(Error::DBus)?
            .path(handler.object_path.clone())
            .map_err(Error::DBus)?
            .build()
            .await
            .map_err(Error::DBus)?;

        proxy
            .authenticate(params)
            .await
            .map_err(|e| Error::AuthenticationFailed {
                reason: format!("provider handler {} failed: {e}", handler.bus_name),
            })
    }

    /// Fetches the account identity from the provider's `[userinfo]` endpoint:
    /// one bearer-authenticated `GET`, then the configured fields are read out
    /// of the JSON response.
    async fn get_user_info(
        &self,
        manifest: &accounts_core::ProviderManifest,
        access_token: &str,
    ) -> Result<UserInfo> {
        let userinfo = manifest
            .userinfo
            .as_ref()
            .ok_or_else(|| Error::AuthenticationFailed {
                reason: format!(
                    "provider {} has no [userinfo] section in its manifest",
                    manifest.provider.id
                ),
            })?;

        let body: serde_json::Value = reqwest::Client::new()
            .get(&userinfo.url)
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let field = |name: &str| {
            body.get(name)
                .and_then(|value| value.as_str())
                .map(str::to_string)
        };

        let email = field(&userinfo.email_field);
        let username = userinfo
            .username_field
            .as_deref()
            .and_then(field)
            .or_else(|| email.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        let display_name = field(&userinfo.display_name_field).unwrap_or_else(|| username.clone());

        Ok(UserInfo {
            display_name,
            username,
            email,
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
