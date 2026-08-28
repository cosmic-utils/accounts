use crate::daemon::{Error, account::AccountInterface, auth::AuthManager};
use accounts_core::config::AccountsConfig;
use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::{
    fdo::Result,
    interface,
    object_server::SignalEmitter,
    zvariant::{ObjectPath, OwnedObjectPath},
};

pub fn account_object_path(id: &uuid::Uuid) -> OwnedObjectPath {
    OwnedObjectPath::try_from(format!(
        "/dev/edfloreshz/Accounts/Accounts/{}",
        id.to_string().replace('-', "_")
    ))
    .expect("account object path is always a valid path")
}

pub struct ManagerInterface {
    pub(crate) config: Arc<Mutex<AccountsConfig>>,
    pub(crate) auth_manager: Arc<Mutex<AuthManager>>,
}

impl ManagerInterface {
    pub fn new(config: Arc<Mutex<AccountsConfig>>, auth_manager: Arc<Mutex<AuthManager>>) -> Self {
        Self {
            config,
            auth_manager,
        }
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Manager")]
impl ManagerInterface {
    async fn list_accounts(&self) -> Result<Vec<OwnedObjectPath>> {
        let config = self.config.lock().await;
        Ok(config
            .accounts
            .iter()
            .map(|account| account_object_path(&account.id))
            .collect())
    }

    async fn list_providers(&self) -> Result<Vec<OwnedObjectPath>> {
        let Some(registry) = crate::daemon::REGISTRY.get() else {
            return Ok(Vec::new());
        };
        Ok(registry
            .list()
            .into_iter()
            .filter_map(|manifest| {
                ObjectPath::try_from(format!(
                    "/dev/edfloreshz/Accounts/Providers/{}",
                    manifest.provider.id
                ))
                .ok()
                .map(|p| p.into())
            })
            .collect())
    }

    #[zbus(property)]
    async fn version(&self) -> Result<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn start_authentication(&self, provider_id: &str) -> Result<String> {
        let Some(registry) = crate::daemon::REGISTRY.get() else {
            return Err(Error::InvalidProviderConfig.into());
        };
        if registry.get(provider_id).is_none() {
            return Err(Error::InvalidProvider(provider_id.to_string()).into());
        }

        let mut auth_manager = self.auth_manager.lock().await;
        match auth_manager.start_auth_flow(provider_id.to_string()).await {
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

    async fn complete_authentication(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        csrf_token: &str,
        authorization_code: &str,
    ) -> Result<OwnedObjectPath> {
        let account = {
            let mut auth_manager = self.auth_manager.lock().await;
            auth_manager
                .complete_auth_flow(csrf_token.to_string(), authorization_code.to_string())
                .await
                .map_err(|err| -> zbus::fdo::Error {
                    Error::AuthenticationFailed {
                        reason: err.to_string(),
                    }
                    .into()
                })?
        };

        {
            let mut config = self.config.lock().await;
            config
                .save_account(&account)
                .map_err(|err| -> zbus::fdo::Error {
                    Error::AccountNotSaved(err.to_string()).into()
                })?;
        }

        let path = account_object_path(&account.id);

        if let Some(connection) = crate::daemon::CONNECTION.get() {
            connection
                .object_server()
                .at(
                    path.clone(),
                    AccountInterface::new(
                        account.id,
                        self.config.clone(),
                        self.auth_manager.clone(),
                    ),
                )
                .await?;

            let services = crate::daemon::services::ServiceFactory::create_services(&account);
            for service in services {
                service.add_service().await?;
            }
        }

        Self::account_added(&emitter, path.clone())
            .await
            .map_err(zbus::fdo::Error::from)?;

        Ok(path)
    }

    async fn emit_authentication_failed(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        reason: &str,
    ) -> Result<()> {
        Self::authentication_failed(&emitter, reason)
            .await
            .map_err(Into::into)
    }

    #[zbus(signal)]
    pub(crate) async fn account_added(
        emitter: &SignalEmitter<'_>,
        account: OwnedObjectPath,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub(crate) async fn account_removed(
        emitter: &SignalEmitter<'_>,
        account: OwnedObjectPath,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn authentication_failed(emitter: &SignalEmitter<'_>, reason: &str) -> zbus::Result<()>;
}
