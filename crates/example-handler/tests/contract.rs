//! Verifies the `ProviderHandler` wire contract the daemon relies on: the
//! `#[proxy]` in `accounts_core` and an `#[interface]` impl line up, `(identity,
//! blob)` round-trips, `login_hint` is honoured, and a refresh call carrying the
//! previous blob under `HANDLER_BLOB_PARAM` gets a rotated blob back.
//!
//! Requires a session bus (`DBUS_SESSION_BUS_ADDRESS`); run under
//! `dbus-run-session` if none is present.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use accounts_core::proxy::{HANDLER_BLOB_PARAM, ProviderHandlerProxy};
use accounts_example_handler::{ExampleHandler, INITIAL_BLOB, OBJECT_PATH, REFRESH_SUFFIX};
use base64::Engine;

static SEQ: AtomicU32 = AtomicU32::new(0);

/// Serves `ExampleHandler` under a bus name unique to this test invocation
/// (tests run concurrently and each owns a name). The returned connection must
/// be kept alive for the duration of the test.
async fn serve() -> (zbus::Connection, String) {
    let name = format!(
        "dev.edfloreshz.Accounts.ProviderHandler.test.p{}s{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let connection = zbus::connection::Builder::session()
        .expect("session bus")
        .name(name.as_str())
        .expect("request name")
        .serve_at(OBJECT_PATH, ExampleHandler)
        .expect("serve interface")
        .build()
        .await
        .expect("build server connection");
    (connection, name)
}

async fn proxy_to(name: &str) -> ProviderHandlerProxy<'static> {
    let client = zbus::Connection::session().await.expect("client bus");
    ProviderHandlerProxy::builder(&client)
        .destination(name.to_owned())
        .expect("destination")
        .path(OBJECT_PATH)
        .expect("path")
        .build()
        .await
        .expect("build proxy")
}

#[tokio::test]
async fn authenticate_returns_identity_and_blob() {
    let (_server, name) = serve().await;
    let proxy = proxy_to(&name).await;

    let (identity, blob) = proxy
        .authenticate(HashMap::new())
        .await
        .expect("authenticate");

    assert_eq!(identity, "device-user@example.com");
    assert_eq!(blob, INITIAL_BLOB);
}

#[tokio::test]
async fn login_hint_param_is_forwarded() {
    let (_server, name) = serve().await;
    let proxy = proxy_to(&name).await;

    let params = HashMap::from([("login_hint".to_string(), "alice@corp.example".to_string())]);
    let (identity, _blob) = proxy.authenticate(params).await.expect("authenticate");

    assert_eq!(identity, "alice@corp.example");
}

#[tokio::test]
async fn refresh_rotates_the_previous_blob() {
    let (_server, name) = serve().await;
    let proxy = proxy_to(&name).await;

    let (_identity, first) = proxy.authenticate(HashMap::new()).await.expect("initial");

    let params = HashMap::from([(
        HANDLER_BLOB_PARAM.to_string(),
        base64::engine::general_purpose::STANDARD.encode(&first),
    )]);
    let (_identity, refreshed) = proxy.authenticate(params).await.expect("refresh");

    let mut expected = first.clone();
    expected.extend_from_slice(REFRESH_SUFFIX);
    assert_eq!(refreshed, expected);
    assert_ne!(refreshed, first);
}
