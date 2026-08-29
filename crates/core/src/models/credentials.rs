pub use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Credential {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scope: Vec<String>,
    pub token_type: String,
    /// Opaque credential material from a `ProviderHandler`. When set, `token_type`
    /// is `"handler"` and `access_token` is empty; `Credentials.GetAccessToken`
    /// hands this back verbatim (as UTF-8) — its shape is a private contract
    /// between the handler and its consumer.
    #[serde(default)]
    pub credential_blob: Option<Vec<u8>>,
}
