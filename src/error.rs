use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccountsError {
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
    Storage(#[from] keyring::Error),

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

impl Into<zbus::fdo::Error> for AccountsError {
    fn into(self) -> zbus::fdo::Error {
        match self {
            AccountsError::AccountNotFound(id) => {
                zbus::fdo::Error::Failed(format!("Account {id} not found."))
            }
            AccountsError::AuthenticationFailed { reason } => zbus::fdo::Error::Failed(reason),
            AccountsError::TokenExpired { account_id } => {
                zbus::fdo::Error::Failed(format!("Token expired for {account_id}"))
            }
            AccountsError::Network(error) => {
                zbus::fdo::Error::Failed(format!("Network error: {error}"))
            }
            AccountsError::OAuth2(request_token_error) => {
                zbus::fdo::Error::Failed(format!("OAuth2 error: {request_token_error}"))
            }
            AccountsError::Serialization(error) => {
                zbus::fdo::Error::Failed(format!("Serialization error: {error}"))
            }
            AccountsError::Storage(error) => {
                zbus::fdo::Error::Failed(format!("Storage error: {error}"))
            }
            AccountsError::DBus(error) => zbus::fdo::Error::Failed(format!("DBus error: {error}")),
            AccountsError::Zbus(error) => zbus::fdo::Error::Failed(format!("Zbus error: {error}")),
            AccountsError::InvalidProviderConfig => {
                zbus::fdo::Error::Failed("Invalid provider configuration".to_string())
            }
            AccountsError::Io(error) => zbus::fdo::Error::Failed(format!("IO error: {error}")),
            AccountsError::UrlParse(parse_error) => {
                zbus::fdo::Error::Failed(format!("URL parse error: {parse_error}"))
            }
            AccountsError::TomlParse(error) => {
                zbus::fdo::Error::Failed(format!("Toml parse error: {error}"))
            }
            AccountsError::InvalidArguments(args) => {
                zbus::fdo::Error::Failed(format!("Invalid arguments: {args}"))
            }
            AccountsError::StorageError(error) => {
                zbus::fdo::Error::Failed(format!("Storage error: {error}"))
            }
            AccountsError::AccountNotSaved(id) => {
                zbus::fdo::Error::Failed(format!("Account not saved: {id}"))
            }
            AccountsError::AccountNotUpdated(id) => {
                zbus::fdo::Error::Failed(format!("Account not updated: {id}"))
            }
            AccountsError::AccountNotRemoved(id) => {
                zbus::fdo::Error::Failed(format!("Account not removed: {id}"))
            }
            AccountsError::TokenRefreshFailed(id) => {
                zbus::fdo::Error::Failed(format!("Token refresh failed for account: {id}"))
            }
            AccountsError::InvalidProvider(name) => {
                zbus::fdo::Error::Failed(format!("Invalid provider: {name}"))
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, AccountsError>;
