//! Coarse (layer 1) authorization via polkit.
//!
//! The spec mandates a two-layer authorization model: this module is layer 1, a
//! system-wide polkit check keyed on a small fixed set of actions. Layer 2 (the
//! per-account, per-service consent grant table) is applied on top of a positive
//! result here and lives elsewhere.

use std::collections::HashMap;

use tokio::sync::OnceCell;
use zbus::message::Header;
use zbus_polkit::policykit1::{AuthorityProxy, CheckAuthorizationFlags, Subject};

/// `auth_self` — gates account mutations (`EnableService`, `DisableService`,
/// `Remove`, and the `DisplayName` property write).
pub const ACTION_MANAGE_OWN_ACCOUNTS: &str = "dev.edfloreshz.Accounts.manage-own-accounts";
/// `allow_active: yes` — gates `GetAccessToken`. Necessary but not sufficient:
/// the layer-2 grant table still applies once this passes.
pub const ACTION_GET_TOKEN: &str = "dev.edfloreshz.Accounts.get-token";
/// `auth_admin` — reserved for the not-yet-implemented `RegisterProvider`.
#[allow(dead_code)]
pub const ACTION_REGISTER_PROVIDER: &str = "dev.edfloreshz.Accounts.register-provider";

static SYSTEM_BUS: OnceCell<zbus::Connection> = OnceCell::const_new();

async fn system_bus() -> zbus::Result<&'static zbus::Connection> {
    SYSTEM_BUS
        .get_or_try_init(zbus::Connection::system)
        .await
}

/// Returns `true` only if polkit positively authorizes the message sender for
/// `action_id`. Any failure to reach or query polkit is treated as a denial
/// (fail closed), matching the spec's stance that credential brokering must not
/// fall back to permissive behaviour when the authority is unavailable.
pub async fn check(header: &Header<'_>, action_id: &str) -> bool {
    match check_inner(header, action_id).await {
        Ok(authorized) => authorized,
        Err(err) => {
            tracing::warn!("polkit check for {action_id} failed, denying: {err}");
            false
        }
    }
}

async fn check_inner(header: &Header<'_>, action_id: &str) -> zbus::Result<bool> {
    let subject = Subject::new_for_message_header(header)
        .map_err(|e| zbus::Error::Failure(format!("could not build polkit subject: {e}")))?;
    let authority = AuthorityProxy::new(system_bus().await?).await?;
    let result = authority
        .check_authorization(
            &subject,
            action_id,
            &HashMap::new(),
            CheckAuthorizationFlags::AllowUserInteraction.into(),
            "",
        )
        .await?;
    Ok(result.is_authorized)
}
