# Architecture

## Overview

`accounts` is a COSMIC desktop service for managing online accounts and their OAuth2 credentials. It follows the GNOME Online Accounts model: a background daemon owns account metadata, secrets, and the OAuth2 flow, and exposes each account's enabled "services" (Calendar, Email, Contacts, Todo) as individual D-Bus objects that other COSMIC applications can consume without ever touching credentials directly. A COSMIC/iced GUI app lets the user add accounts and toggle which services are enabled.

Providers (Google, Microsoft, and any others) are not compiled into the daemon. Each provider is a separate process, declared by a manifest and speaking a small D-Bus interface — see [PROVIDER_PLUGINS.md](PROVIDER_PLUGINS.md) for the full design.

## Workspace layout

The repo is a Cargo workspace with five crates:

| Crate | Location | Role |
|---|---|---|
| `accounts` | `src/` (workspace root package) | Shared core library: domain models, D-Bus proxy definitions, typed client wrappers, config persistence, the `AccountService` trait, the provider manifest registry |
| `accounts-daemon` | `accounts-daemon/` | Background service binary: OAuth2/PKCE flow, D-Bus server, secret storage, HTTP OAuth callback listener (axum) |
| `accounts-provider-google` | `accounts-provider-google/` | Out-of-process provider for Google, implementing `Provider1` |
| `accounts-provider-microsoft` | `accounts-provider-microsoft/` | Out-of-process provider for Microsoft, implementing `Provider1` |
| `accounts-ui` | `accounts-ui/` | COSMIC/iced GUI binary; a pure D-Bus client of the daemon, holds no secrets |

## Core library (`accounts`)

- **`src/models/`** — domain types shared by daemon and UI:
  - `account.rs` — `Account` (id, provider, display_name, username, email, enabled, timestamps, per-service enabled map) and its D-Bus wire form `DbusAccount`, with `From`/`Into` conversions. `Account::dbus_id()` converts the UUID into a D-Bus-object-path-safe string.
  - `provider.rs` — `Provider` is a type alias for `String`: a provider id (e.g. `"google"`) resolved against the manifest registry, not a closed set compiled into this crate.
  - `provider_info.rs` — `DbusProviderInfo { id, name, services }`, the capability-advertisement payload returned by `list_providers()`.
  - `service.rs` — `Service` enum (`Email`, `Calendar`, `Contacts`, `Todo`) and its `DbusService` counterpart.
  - `credentials.rs` — `Credential { access_token, refresh_token, expires_at, scope, token_type }`.
- **`src/registry.rs`** — `ProviderRegistry`: loads `ProviderManifest`s (id, name, D-Bus name, services, OAuth endpoints/scopes/client credentials) from XDG data dirs (`ProviderRegistry::search_dirs()`), independent of whether any provider process is running.
- **`src/proxy.rs`** — zbus `#[proxy]` trait definitions consumed by clients: `Accounts` (interface `dev.edfloreshz.Accounts.Account`, well-known bus name `dev.edfloreshz.Accounts`) for account management and capability advertisement (`list_providers`), `Calendar` for the per-account calendar service, and `Provider1` (interface `dev.edfloreshz.Accounts.Provider1`) implemented by each provider process — `get_user_info` and `get_service_config`, both provider-agnostic on the daemon side.
- **`src/clients/`** — typed async wrappers over the proxies, re-exported as `AccountsClient`:
  - `account.rs` — `AccountsClient`: list/get/remove accounts, list providers, start/complete OAuth authentication, enable account/service, get access/refresh tokens, ensure credentials, plus D-Bus signal emit/receive helpers used by the UI's live-update subscriptions.
  - `calendar.rs` — `CalendarClient`: per-account calendar proxy at `/dev/edfloreshz/Accounts/Calendar/{account.dbus_id()}`.
- **`src/service.rs`** — the `AccountService` async trait that unifies per-service D-Bus objects implemented by the daemon (`name()`, `interface_name()`, `is_supported()`, `get_config()`, `add_service()`, `remove_service()`, `ensure_credentials()`).
- **`src/config.rs`** — `AccountsConfig { accounts: Vec<Account> }`, persisted via `cosmic-config` under app id `dev.edfloreshz.AccountsDaemon`. This is the account **metadata** store, distinct from secret storage.

## Daemon (`accounts-daemon`)

- **`main.rs`** — entry point. Starts an axum HTTP server on `127.0.0.1:8080` with a `/callback` route for the OAuth2 redirect, loads the `ProviderRegistry` into the static `REGISTRY`, opens a zbus session connection, registers the `AccountsInterface` D-Bus object under the well-known name `dev.edfloreshz.Accounts`, then instantiates and registers per-service D-Bus objects for each persisted, enabled account via `ServiceFactory`. `handle_callback` exchanges the OAuth `code`/`state`, completes auth via `AccountsClient`, emits `account_added`/`account_exists` signals, and returns an HTML result page.
- **`account.rs`** — `AccountsInterface`, the zbus `#[interface(name = "dev.edfloreshz.Accounts.Account")]` implementation clients talk to: list/get/remove accounts, list providers (served straight from `REGISTRY`, no provider process involved), enable account/service, start/complete auth, get access/refresh tokens, plus signal emitters (`account_added`, `account_removed`, `account_changed`, `account_exists`).
- **`auth.rs`** — `AuthManager`: holds its own session-bus `Connection` (separate from the object-server one) used to call provider processes, and tracks in-flight PKCE exchanges keyed by CSRF token. Contains no provider-specific code.
  - `start_auth_flow(provider_id)` looks up the manifest in `REGISTRY`, builds an `oauth2::basic::BasicClient` from its `[oauth]` section (including any `extra_params`, e.g. Google's `access_type=offline`), generates a PKCE challenge, and returns an authorize URL (opened by the UI via `open::that_detached`).
  - `complete_auth_flow(csrf_token, code)` exchanges the code for tokens, then calls `Provider1::get_user_info` on the account's provider process over D-Bus, checks for duplicate accounts, and stores the resulting `Credential` via `CredentialStorage`.
  - `refresh_token`/`ensure_credentials` refresh access tokens once `expires_at` has passed, using the manifest's OAuth config — fully provider-agnostic.
- **`storage.rs`** — `CredentialStorage`, backed by the freedesktop Secret Service API (`secret-service` crate, i.e. GNOME Keyring/KWallet). Stores/retrieves/deletes serialized `Credential` JSON keyed by account UUID. This is the **secrets** store, separate from `AccountsConfig`. Providers never see it — they only ever receive a short-lived access token as a call argument.
- **`services/`** — per-service D-Bus objects implementing the core `AccountService` trait, with GOA-style property names (e.g. `ImapHost`, `SmtpAuthXoauth2`):
  - `mod.rs` — `ServiceFactory` creates and registers service objects for an account. **Only `Calendar` is currently wired up**; `mail.rs`, `contacts.rs`, and `todo.rs` still exist but are commented out and still reference the old provider enum — future work, not touched by the provider-plugin migration.
  - `calendar.rs` — `CalendarService`, interface `dev.edfloreshz.Accounts.Calendar`. `uri`/`accept_ssl_errors` are no longer hardcoded: they come from a live `Provider1::get_service_config("calendar")` D-Bus call to the account's provider process, resolved via `REGISTRY`.
- **`data/`** — `cosmic-accounts.service` (systemd user unit, D-Bus-activated, `BusName=dev.edfloreshz.Accounts`) and `providers/{google,microsoft}.toml` (provider manifests — also the dev-mode fallback entry in `ProviderRegistry::search_dirs()`, so `cargo run` works without installing anything).

## UI (`accounts-ui`)

A single-crate COSMIC/iced app using the Elm architecture, implemented almost entirely in `app.rs`:

- **State (`AppModel`)** — `cosmic::Core`, nav bar (one entry per account), dialog stack (Add Account), toasts, an `Option<AccountsClient>`, the loaded `Vec<Account>`, and `Vec<DbusProviderInfo>` fetched from `client.list_providers()` after the client connects.
- **Views** — `welcome_view()` (provider picker shown with no account selected), `add_account_dialog()`, `account_view()` (account details plus per-service togglers), and an `about()` context page (repo link, embedded git SHA/date via `vergen`).
- **Subscriptions** — long-lived streams over `AccountsClient::receive_account_added/changed/removed/exists` translate daemon-side D-Bus signals into UI `Message`s, keeping the UI in sync after OAuth completes out-of-process in the daemon's HTTP callback handler.
- **`update()`** — `StartAuth(provider)` calls the daemon to begin auth and opens the returned URL in the system browser; account/service toggles call the client then reload state.
- **`i18n.rs`** — Fluent-based localization (`i18n-embed` + `i18n-embed-fl`), with translations at `i18n/en/accounts_ui.ftl` (source of truth) and `i18n/pl/accounts_ui.ftl`. Language is selected automatically via `DesktopLanguageRequester`.

## Provider abstraction

Providers run out-of-process. `Provider` is a plain `String` id (e.g. `"google"`), not an enum — there is no compiled-in provider knowledge in `accounts-daemon` at all. See [PROVIDER_PLUGINS.md](PROVIDER_PLUGINS.md) for the full design; summary:

1. A **manifest** (`ProviderManifest`, TOML) declares the provider's id, display name, D-Bus name, supported services, and OAuth endpoints/scopes/client credentials. Loaded by `ProviderRegistry` from XDG data dirs at daemon startup — this alone is enough to answer "which providers exist and what do they support" (`list_providers()`), without any provider process running.
2. A **provider process** implements the `Provider1` D-Bus interface (`get_user_info`, `get_service_config`) and registers itself under the manifest's `dbus_name`. `accounts-daemon` calls it only when it actually needs provider-specific data (fetching user info during auth, resolving a service's connection config), and only ever hands it a short-lived access token — never a refresh token or the credential store.

`accounts-provider-google` and `accounts-provider-microsoft` are the first two providers, implemented this way with no special-casing. Adding a third-party provider means shipping a manifest, a `.service` D-Bus activation file, and a binary — no changes to this repo.

The OAuth2/PKCE/CSRF flow itself stays fully generic via the `oauth2` crate, driven entirely by the manifest's `[oauth]` section.

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
3. **Provider manifests** (client id/secret, endpoints, scopes, D-Bus name, services) — TOML files under XDG data dirs (`ProviderRegistry::search_dirs()`: `$XDG_DATA_HOME`/`$XDG_DATA_DIRS`/`/usr/share`/`/usr/local/share`, plus a repo-relative `accounts-daemon/data/providers` dev fallback so `cargo run` works without installing).

## Build & packaging

- **Root `justfile`** — workspace-wide dev tasks: `build`/`build-lib`/`build-daemon`/`build-gui`, `lint`, `format`, `install`/`uninstall` (installs to `/usr/bin`, `/usr/share/dbus-1/services/`, `/etc/accounts/providers`), `start-daemon`/`stop-daemon`/`logs` (systemd `--user`). Some recipes (`test`, `cli-*`, `example-*`) reference `tests/integration_test`, `examples/cli.rs`, and `examples/daemon.rs` that don't currently exist in the tree — likely planned/aspirational tooling.
- **`accounts-ui/justfile`** — the packaging-grade recipe set used for distribution: `build-release`, `build-vendored` (offline `cargo vendor`), `install`/`uninstall` (binary + `.desktop` + `.metainfo.xml` + icon under a configurable prefix), embeds `SOURCE_DATE_EPOCH`/`SOURCE_GIT_HASH` for reproducible builds.
- **systemd** — `accounts-daemon/data/cosmic-accounts.service`, D-Bus-activated user unit.
- **Desktop integration** — `accounts-ui/resources/app.desktop` and `app.metainfo.xml` for launcher/software-center listing.
- No Flatpak manifest currently exists in the repo; packaging is distro-native only for now.

## Known gaps

- `mail.rs`, `contacts.rs`, and `todo.rs` service modules are commented out in `accounts-daemon/src/services/mod.rs` and still reference the old provider enum, so they won't compile as-is if re-enabled — only Calendar was migrated to the provider-plugin model.
- Root `justfile` references `examples/cli.rs`, `examples/daemon.rs`, `tests/integration_test`, and the `install-configs` target still targets `/etc/accounts/providers` rather than the XDG data dirs `ProviderRegistry` actually searches — needs updating to match.
- The `Calendar` zbus proxy in `src/proxy.rs` declares `default_service = "dev.edfloreshz.Accounts.Calendar"`, which looks inconsistent with the bus name (`dev.edfloreshz.Accounts`) used everywhere else — worth verifying against actual runtime behavior.
- No D-Bus policy files restricting who can own `dev.edfloreshz.Accounts.Provider.*` names yet — see the Security notes in [PROVIDER_PLUGINS.md](PROVIDER_PLUGINS.md).
