//! The layer-2 consent prompt: when `Credentials.GetAccessToken` has no stored
//! grant for a caller, the daemon asks the user. The prompt UI is a separate,
//! D-Bus-activated process (`dev.edfloreshz.Accounts.ConsentPrompt`) so the
//! credentials daemon itself stays headless and toolkit-agnostic.

use std::time::Duration;

/// How long `GetAccessToken` blocks waiting for the user to answer before it
/// gives up with `Error.ConsentTimeout`.
pub const CONSENT_TIMEOUT: Duration = Duration::from_secs(120);

pub enum ConsentError {
    Timeout,
    Failed(String),
}

#[zbus::proxy(
    interface = "dev.edfloreshz.Accounts.ConsentPrompt",
    default_service = "dev.edfloreshz.Accounts.ConsentPrompt",
    default_path = "/dev/edfloreshz/Accounts/ConsentPrompt",
    gen_blocking = false
)]
trait ConsentPrompt {
    /// Shows the prompt and blocks until the user chooses. `true` = allow.
    fn prompt(
        &self,
        caller_name: &str,
        account_name: &str,
        provider_id: &str,
        service: &str,
    ) -> zbus::Result<bool>;
}

/// Asks the user whether `caller_name` may use `account_name`'s `service`
/// credentials. D-Bus-activates the helper on first use.
pub async fn prompt(
    connection: &zbus::Connection,
    caller_name: &str,
    account_name: &str,
    provider_id: &str,
    service: &str,
) -> std::result::Result<bool, ConsentError> {
    let proxy = ConsentPromptProxy::new(connection)
        .await
        .map_err(|e| ConsentError::Failed(e.to_string()))?;

    match tokio::time::timeout(
        CONSENT_TIMEOUT,
        proxy.prompt(caller_name, account_name, provider_id, service),
    )
    .await
    {
        Ok(Ok(decision)) => Ok(decision),
        Ok(Err(e)) => Err(ConsentError::Failed(e.to_string())),
        Err(_) => Err(ConsentError::Timeout),
    }
}
