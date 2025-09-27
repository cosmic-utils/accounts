use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Account not saved: {0}")]
    AccountNotSaved(String),

    #[error("Account not updated: {0}")]
    AccountNotUpdated(String),

    #[error("Account not removed: {0}")]
    AccountNotRemoved(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("Token expired for account: {account_id}")]
    TokenExpired { account_id: String },

    #[error("Token refresh failed for account: {0}")]
    TokenRefreshFailed(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("OAuth2 error: {0}")]
    OAuth2(
        #[from]
        oauth2::RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
        >,
    ),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Storage error: {0}")]
    CredentialStorage(#[from] keyring::Error),

    #[error("Cosmic Config error: {0}")]
    CosmicConfig(#[from] cosmic_config::Error),

    #[error("D-Bus error: {0}")]
    DBus(#[from] zbus::Error),

    #[error("Zbus error: {0}")]
    Zbus(#[from] zbus::fdo::Error),

    #[error("Invalid provider configuration")]
    InvalidProviderConfig,

    #[error("Invalid provider {0}")]
    InvalidProvider(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),
}

impl Into<zbus::fdo::Error> for Error {
    fn into(self) -> zbus::fdo::Error {
        match self {
            Error::AccountNotFound(id) => {
                zbus::fdo::Error::Failed(format!("Account {id} not found."))
            }
            Error::AuthenticationFailed { reason } => zbus::fdo::Error::Failed(reason),
            Error::TokenExpired { account_id } => {
                zbus::fdo::Error::Failed(format!("Token expired for {account_id}"))
            }
            Error::Network(error) => zbus::fdo::Error::Failed(format!("Network error: {error}")),
            Error::OAuth2(request_token_error) => {
                zbus::fdo::Error::Failed(format!("OAuth2 error: {request_token_error}"))
            }
            Error::Serialization(error) => {
                zbus::fdo::Error::Failed(format!("Serialization error: {error}"))
            }
            Error::CredentialStorage(error) => {
                zbus::fdo::Error::Failed(format!("Credential storage error: {error}"))
            }
            Error::CosmicConfig(error) => {
                zbus::fdo::Error::Failed(format!("Cosmic config error: {error}"))
            }
            Error::DBus(error) => zbus::fdo::Error::Failed(format!("DBus error: {error}")),
            Error::Zbus(error) => zbus::fdo::Error::Failed(format!("Zbus error: {error}")),
            Error::InvalidProviderConfig => {
                zbus::fdo::Error::Failed("Invalid provider configuration".to_string())
            }
            Error::Io(error) => zbus::fdo::Error::Failed(format!("IO error: {error}")),
            Error::UrlParse(parse_error) => {
                zbus::fdo::Error::Failed(format!("URL parse error: {parse_error}"))
            }
            Error::TomlParse(error) => {
                zbus::fdo::Error::Failed(format!("Toml parse error: {error}"))
            }
            Error::InvalidArguments(args) => {
                zbus::fdo::Error::Failed(format!("Invalid arguments: {args}"))
            }
            Error::StorageError(error) => {
                zbus::fdo::Error::Failed(format!("Storage error: {error}"))
            }
            Error::AccountNotSaved(id) => {
                zbus::fdo::Error::Failed(format!("Account not saved: {id}"))
            }
            Error::AccountNotUpdated(id) => {
                zbus::fdo::Error::Failed(format!("Account not updated: {id}"))
            }
            Error::AccountNotRemoved(id) => {
                zbus::fdo::Error::Failed(format!("Account not removed: {id}"))
            }
            Error::TokenRefreshFailed(id) => {
                zbus::fdo::Error::Failed(format!("Token refresh failed for account: {id}"))
            }
            Error::InvalidProvider(name) => {
                zbus::fdo::Error::Failed(format!("Invalid provider: {name}"))
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
