use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccountsError {
    #[error("Account not found: {id}")]
    AccountNotFound { id: String },

    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("Token expired for account: {account_id}")]
    TokenExpired { account_id: String },

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

    #[error("Invalid provider configuration")]
    InvalidProviderConfig,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, AccountsError>;
