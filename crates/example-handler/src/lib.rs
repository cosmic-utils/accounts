// SPDX-License-Identifier: GPL-3.0-only

//! A reference `dev.edfloreshz.Accounts.ProviderHandler` — the extension point
//! for providers whose auth flow the daemon can't drive itself. This one fakes
//! a device-code flow: it "issues" an opaque credential blob and, when handed a
//! previous blob back on refresh, returns a rotated one.
//!
//! It exists to exercise the daemon's handler path end to end; a real handler
//! (device-code, enterprise SSO, smartcard) would ship as its own project and
//! be pointed at from a provider manifest's `[handler]` section.

use std::collections::HashMap;

use accounts_core::proxy::HANDLER_BLOB_PARAM;
use base64::Engine;

pub const BUS_NAME: &str = "dev.edfloreshz.Accounts.ProviderHandler.Example";
pub const OBJECT_PATH: &str = "/dev/edfloreshz/Accounts/ProviderHandler";

/// The blob a fresh `Authenticate` hands back.
pub const INITIAL_BLOB: &[u8] = b"example-handler-credential-v1";
/// Appended to the previous blob on every refresh, so rotation is observable.
pub const REFRESH_SUFFIX: &[u8] = b"+refreshed";

pub struct ExampleHandler;

#[zbus::interface(name = "dev.edfloreshz.Accounts.ProviderHandler")]
impl ExampleHandler {
    /// `params` is whatever `Manager.CreateAccount` was given, passed straight
    /// through by the daemon — plus, on a refresh, the previous blob under
    /// `HANDLER_BLOB_PARAM` (base64).
    async fn authenticate(
        &self,
        params: HashMap<String, String>,
    ) -> zbus::fdo::Result<(String, Vec<u8>)> {
        let identity = params
            .get("login_hint")
            .cloned()
            .unwrap_or_else(|| "device-user@example.com".to_string());

        if let Some(encoded) = params.get(HANDLER_BLOB_PARAM) {
            let mut blob = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| {
                    zbus::fdo::Error::InvalidArgs(format!("bad {HANDLER_BLOB_PARAM}: {e}"))
                })?;
            blob.extend_from_slice(REFRESH_SUFFIX);
            tracing::info!(%identity, "refreshed credential for handler account");
            return Ok((identity, blob));
        }

        tracing::info!(%identity, "issued credential for new handler account");
        Ok((identity, INITIAL_BLOB.to_vec()))
    }
}

/// Owns [`BUS_NAME`] on the session bus and serves [`ExampleHandler`] until the
/// process is killed.
pub async fn run() -> zbus::Result<()> {
    let _connection = zbus::connection::Builder::session()?
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, ExampleHandler)?
        .build()
        .await?;
    tracing::info!("{BUS_NAME} ready at {OBJECT_PATH}");
    std::future::pending::<()>().await;
    Ok(())
}
