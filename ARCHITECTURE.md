# Architecture

## Overview

`accounts` is a COSMIC desktop service for managing online accounts (currently Google and Microsoft) and their OAuth2 credentials. It follows the GNOME Online Accounts model: a background daemon owns account metadata, secrets, and the OAuth2 flow, and exposes each account's enabled "services" (Calendar, Email, Contacts, Todo) as individual D-Bus objects that other COSMIC applications can consume without ever touching credentials directly. A COSMIC/iced GUI app lets the user add accounts and toggle which services are enabled.

## Workspace layout

The repo is a Cargo workspace with three crates:

| Crate | Location | Role |
|---|---|---|
| `accounts` | `src/` (workspace root package) | Shared core library: domain models, D-Bus proxy definitions, typed client wrappers, config persistence, the `AccountService` trait |
| `accounts-daemon` | `accounts-daemon/` | Background service binary: OAuth2/PKCE flow, D-Bus server, secret storage, HTTP OAuth callback listener (axum) |
| `accounts-ui` | `accounts-ui/` | COSMIC/iced GUI binary; a pure D-Bus client of the daemon, holds no secrets |

## Core library (`accounts`)

- **`src/models/`** — domain types shared by daemon and UI:
  - `account.rs` — `Account` (id, provider, display_name, username, email, enabled, timestamps, per-service enabled map) and its D-Bus wire form `DbusAccount`, with `From`/`Into` conversions. `Account::dbus_id()` converts the UUID into a D-Bus-object-path-safe string.
  - `provider.rs` — `Provider` enum (`Google`, `Microsoft`) with `from_str`, `list()`, `file_name()` (maps to `google.toml`/`microsoft.toml`), and `services()` (default enabled services per provider).
  - `service.rs` — `Service` enum (`Email`, `Calendar`, `Contacts`, `Todo`) and its `DbusService` counterpart.
  - `credentials.rs` — `Credential { access_token, refresh_token, expires_at, scope, token_type }`.
- **`src/proxy.rs`** — zbus `#[proxy]` trait definitions consumed by clients: `Accounts` (interface `dev.edfloreshz.Accounts.Account`, well-known bus name `dev.edfloreshz.Accounts`) for account management, and `Calendar` for the per-account calendar service.
- **`src/clients/`** — typed async wrappers over the proxies, re-exported as `AccountsClient`:
  - `account.rs` — `AccountsClient`: list/get/remove accounts, start/complete OAuth authentication, enable account/service, get access/refresh tokens, ensure credentials, plus D-Bus signal emit/receive helpers used by the UI's live-update subscriptions.
  - `calendar.rs` — `CalendarClient`: per-account calendar proxy at `/dev/edfloreshz/Accounts/Calendar/{account.dbus_id()}`.
- **`src/service.rs`** — the `AccountService` async trait that unifies per-service D-Bus objects implemented by the daemon (`name()`, `interface_name()`, `is_supported()`, `get_config()`, `add_service()`, `remove_service()`, `ensure_credentials()`).
- **`src/config.rs`** — `AccountsConfig { accounts: Vec<Account> }`, persisted via `cosmic-config` under app id `dev.edfloreshz.AccountsDaemon`. This is the account **metadata** store, distinct from secret storage.

## Daemon (`accounts-daemon`)

- **`main.rs`** — entry point. Starts an axum HTTP server on `127.0.0.1:8080` with a `/callback` route for the OAuth2 redirect, opens a zbus session connection, registers the `AccountsInterface` D-Bus object under the well-known name `dev.edfloreshz.Accounts`, then instantiates and registers per-service D-Bus objects for each persisted, enabled account via `ServiceFactory`. `handle_callback` exchanges the OAuth `code`/`state`, completes auth via `AccountsClient`, emits `account_added`/`account_exists` signals, and returns an HTML result page.
- **`account.rs`** — `AccountsInterface`, the zbus `#[interface(name = "dev.edfloreshz.Accounts.Account")]` implementation clients talk to: list/get/remove accounts, enable account/service, start/complete auth, get access/refresh tokens, plus signal emitters (`account_added`, `account_removed`, `account_changed`, `account_exists`).
- **`auth.rs`** — `AuthManager`: loads per-provider OAuth config from TOML files (`accounts-daemon/data/providers/{provider}.toml`) and tracks in-flight PKCE exchanges keyed by CSRF token.
  - `start_auth_flow(provider)` builds an `oauth2::basic::BasicClient`, generates a PKCE challenge, and returns an authorize URL (opened by the UI via `open::that_detached`).
  - `complete_auth_flow(csrf_token, code)` exchanges the code for tokens, fetches provider user info (Google userinfo endpoint / Microsoft Graph `/me`), checks for duplicate accounts, and stores the resulting `Credential` via `CredentialStorage`.
  - `refresh_token`/`ensure_credentials` refresh access tokens once `expires_at` has passed.
- **`storage.rs`** — `CredentialStorage`, backed by the freedesktop Secret Service API (`secret-service` crate, i.e. GNOME Keyring/KWallet). Stores/retrieves/deletes serialized `Credential` JSON keyed by account UUID. This is the **secrets** store, separate from `AccountsConfig`.
- **`services/`** — per-service D-Bus objects implementing the core `AccountService` trait, with GOA-style property names (e.g. `ImapHost`, `SmtpAuthXoauth2`):
  - `mod.rs` — `ServiceFactory` creates and registers service objects for an account. **Only `Calendar` is currently wired up**; `mail.rs`, `contacts.rs`, and `todo.rs` are implemented but commented out, pending future work.
  - `calendar.rs` — `CalendarService`, interface `dev.edfloreshz.Accounts.Calendar`, exposing `uri` (CalDAV endpoint, provider-specific) and `accept_ssl_errors`.
- **`data/`** — `cosmic-accounts.service` (systemd user unit, D-Bus-activated, `BusName=dev.edfloreshz.Accounts`) and `providers/{google,microsoft}.toml` (OAuth client id/secret, auth/token URLs, scopes).

## UI (`accounts-ui`)

A single-crate COSMIC/iced app using the Elm architecture, implemented almost entirely in `app.rs`:

- **State (`AppModel`)** — `cosmic::Core`, nav bar (one entry per account), dialog stack (Add Account), toasts, an `Option<AccountsClient>`, the loaded `Vec<Account>`, and `Provider::list()`.
- **Views** — `welcome_view()` (provider picker shown with no account selected), `add_account_dialog()`, `account_view()` (account details plus per-service togglers), and an `about()` context page (repo link, embedded git SHA/date via `vergen`).
- **Subscriptions** — long-lived streams over `AccountsClient::receive_account_added/changed/removed/exists` translate daemon-side D-Bus signals into UI `Message`s, keeping the UI in sync after OAuth completes out-of-process in the daemon's HTTP callback handler.
- **`update()`** — `StartAuth(provider)` calls the daemon to begin auth and opens the returned URL in the system browser; account/service toggles call the client then reload state.
- **`i18n.rs`** — Fluent-based localization (`i18n-embed` + `i18n-embed-fl`), with translations at `i18n/en/accounts_ui.ftl` (source of truth) and `i18n/pl/accounts_ui.ftl`. Language is selected automatically via `DesktopLanguageRequester`.

## Provider abstraction

`Provider` is a plain enum (`Google`, `Microsoft`), not a trait — there is no per-provider Rust implementation to plug in. Provider-specific behavior is expressed as:

1. A TOML config file per provider (`accounts-daemon/data/providers/*.toml`) supplying OAuth endpoints, scopes, and client credentials, loaded into a `HashMap<Provider, ProviderConfig>` at daemon startup.
2. `match provider { ... }` arms scattered in `auth.rs` (user-info endpoint and JSON field mapping, Google's `access_type=offline` param) and in `services/*.rs` (CalDAV/IMAP/SMTP/CardDAV/Tasks URIs).

The OAuth2/PKCE/CSRF flow itself is fully generic via the `oauth2` crate. Adding a provider means: add a TOML file, add an enum variant with `Display`/`from_str`/`file_name`/`services()` arms, and add match arms in `auth.rs::get_user_info` plus any `services/*.rs` you want to support.

## Daemon/UI separation & D-Bus contract

```
accounts-ui  <--D-Bus (dev.edfloreshz.Accounts)-->  accounts-daemon  <--OAuth2/HTTPS-->  Google / Microsoft
   (no secrets)                                    (owns config + secrets)
```

The daemon is the sole source of truth: it owns `AccountsConfig` (metadata) and `CredentialStorage` (secrets), performs all OAuth network calls, and exposes the `Account` interface plus one object per enabled service (e.g. `/dev/edfloreshz/Accounts/Calendar/{id}`). The UI is a pure D-Bus client — it never stores or transmits credentials, and its only "network" action is opening the daemon-provided authorize URL in the system browser. The `accounts` core crate is the shared contract: models, proxy/client glue, and the `AccountService` trait the daemon's service objects implement. Other COSMIC apps that want read access to accounts or services should depend only on `accounts::clients::{AccountsClient, CalendarClient}` (or talk to service D-Bus objects directly), never on `accounts-daemon` internals.

## Persistence

Three separate persistence layers:

1. **Account metadata** (non-secret) — `AccountsConfig` via `cosmic-config`, config id `dev.edfloreshz.AccountsDaemon`.
2. **Secrets** (OAuth tokens) — `CredentialStorage` via the Secret Service API (`secret-service` crate → GNOME Keyring/KWallet), keyed by account UUID.
3. **Provider OAuth app config** (client id/secret, endpoints, scopes) — plain TOML files at `accounts-daemon/data/providers/*.toml`, currently loaded by a path relative to the daemon's working directory rather than an XDG-standard location — fragile for installed/packaged deployments.

## Build & packaging

- **Root `justfile`** — workspace-wide dev tasks: `build`/`build-lib`/`build-daemon`/`build-gui`, `lint`, `format`, `install`/`uninstall` (installs to `/usr/bin`, `/usr/share/dbus-1/services/`, `/etc/accounts/providers`), `start-daemon`/`stop-daemon`/`logs` (systemd `--user`). Some recipes (`test`, `cli-*`, `example-*`) reference `tests/integration_test`, `examples/cli.rs`, and `examples/daemon.rs` that don't currently exist in the tree — likely planned/aspirational tooling.
- **`accounts-ui/justfile`** — the packaging-grade recipe set used for distribution: `build-release`, `build-vendored` (offline `cargo vendor`), `install`/`uninstall` (binary + `.desktop` + `.metainfo.xml` + icon under a configurable prefix), embeds `SOURCE_DATE_EPOCH`/`SOURCE_GIT_HASH` for reproducible builds.
- **systemd** — `accounts-daemon/data/cosmic-accounts.service`, D-Bus-activated user unit.
- **Desktop integration** — `accounts-ui/resources/app.desktop` and `app.metainfo.xml` for launcher/software-center listing.
- No Flatpak manifest currently exists in the repo; packaging is distro-native only for now.

## Known gaps

- `mail.rs`, `contacts.rs`, and `todo.rs` service modules are fully implemented but commented out in `accounts-daemon/src/services/mod.rs` — only Calendar is live.
- Root `justfile` references `examples/cli.rs`, `examples/daemon.rs`, and `tests/integration_test`, none of which exist yet.
- Provider TOML files are loaded via a repo-relative path, not an XDG-standard installed location, even though the root `justfile`'s `install-configs` recipe installs them to `/etc/accounts/providers` — the daemon doesn't currently read from there.
- The `Calendar` zbus proxy in `src/proxy.rs` declares `default_service = "dev.edfloreshz.Accounts.Calendar"`, which looks inconsistent with the bus name (`dev.edfloreshz.Accounts`) used everywhere else — worth verifying against actual runtime behavior.
