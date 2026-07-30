# Plan: Out-of-process provider plugins

## Context

Provider support (Google, Microsoft) is currently hardcoded: `Provider` is a closed enum, and provider-specific logic (OAuth endpoints, user-info parsing, per-service URIs) is scattered across `match` arms in `accounts-daemon/src/auth.rs` and `services/*.rs`. Adding a provider requires editing `accounts-daemon` directly, and any provider's code runs in the same process that holds every account's OAuth secrets.

This plan replaces that with providers running as separate processes, each exposing a small D-Bus interface, following the same model COSMIC itself uses for panel applets (`cosmic-panel` spawns applets as independent processes registered by app ID, rather than loading them as libraries — see `ARCHITECTURE.md`). Third parties can ship a provider as an installable package, `accounts-daemon` never links or trusts foreign code in-process, and a provider crash/bug can't touch other accounts' secrets.

There is no live deployment and no existing users to migrate — this rewrites the provider mechanism directly rather than staging it behind compatibility shims.

## Goals

- A provider is a standalone binary + a manifest, discoverable without recompiling `accounts-daemon`.
- `accounts-daemon` talks to providers only over D-Bus — no dynamic library loading, no shared memory, no trust assumptions about provider code.
- Secrets never leave `accounts-daemon`: providers receive a token when they need to make an authenticated call, they never persist or manage storage themselves.
- Google and Microsoft ship as ordinary providers using this exact mechanism — no built-in/first-party shortcut.
- **Applications can discover, ahead of adding an account, which providers support which services** (e.g. "does anything here offer Tasks?") — this is capability *advertisement*, not data fetching. Consumers still talk to the per-account service D-Bus objects (`CalendarService`, etc.) for actual data, exactly as today.

### Out of scope

A unified, normalized data-fetching API (e.g. a `ListTasks(account_id)` that returns provider-agnostic `Task` structs) is explicitly **not** part of this plan. That would be a separate, later effort layered on top of whichever services a provider advertises. This plan only covers: (a) running provider code out-of-process, and (b) telling consumers what a provider is capable of.

## Design

### 1. Provider manifest

A TOML file (mirrors how `.desktop` files register applets), installed under XDG data dirs (e.g. `/usr/share/accounts/providers/*.toml`, `$XDG_DATA_HOME/accounts/providers/*.toml`) — replaces the current relative-path-loaded `accounts-daemon/data/providers/*.toml`:

```toml
[provider]
id = "google"
name = "Google"
dbus_name = "dev.edfloreshz.Accounts.Provider.Google"
exec = "/usr/bin/accounts-provider-google"       # spawned on demand, or activated via D-Bus service activation
services = ["email", "calendar", "contacts"]

[oauth]
auth_url = "https://accounts.google.com/o/oauth2/v2/auth"
token_url = "https://oauth2.googleapis.com/token"
scopes = ["openid", "email", "https://www.googleapis.com/auth/calendar"]
```

Static OAuth metadata (auth/token URL, scopes) stays declarative in the manifest — it doesn't need a running process to be read, so `accounts-daemon` can build the PKCE authorize URL itself without invoking the provider process. This keeps the common OAuth2/PKCE flow (already generic via the `oauth2` crate) inside `accounts-daemon`, rather than duplicating it in every provider.

`Provider` (`src/models/provider.rs`) stops being a closed enum and becomes a `String` id resolved against the registry; `DbusAccount`/`Account.provider` carry that string directly.

### 2. Advertising provider capabilities to applications

The manifest's `services` list (§1) is the single source of truth for "which providers support which services" — static data, readable without launching the provider process or making any D-Bus call to it. Add one read-only method to the `Accounts` D-Bus interface (`src/proxy.rs`):

```rust
// Returns every known provider (from the manifest registry) and the services it declares support for.
// Does not require the provider process to be running.
async fn list_providers(&self) -> Result<Vec<DbusProviderInfo>>;
// DbusProviderInfo { id: String, name: String, services: Vec<DbusService> }
```

`AccountsInterface` (`accounts-daemon/src/account.rs`) serves this straight from `ProviderRegistry` (§4) — no provider IPC involved. `accounts-ui`'s provider picker (`welcome_view()`/`add_account_dialog()`) calls this instead of `Provider::list()`, so any provider with an installed manifest shows up in the "Add Account" grid automatically. Other consumer apps get the same query for free — e.g. a task-management app can call `list_providers()` up front and only prompt "connect an account" if at least one provider advertises `Todo`.

This is the piece you asked for, and it only needs §1 and §4 (the registry) to exist — it doesn't depend on §3/§5/§6 (actually running provider processes) being finished.

### 3. Provider D-Bus interface (`dev.edfloreshz.Accounts.Provider1`)

Each provider process implements one interface, kept intentionally small — only the parts that are genuinely provider-specific:

```rust
#[interface(name = "dev.edfloreshz.Accounts.Provider1")]
trait Provider {
    // Given a fresh access token, return normalized user info (display name, email, username)
    async fn get_user_info(&self, access_token: &str) -> Result<HashMap<String, String>>;

    // Given a service id ("calendar", "todo", "email"), return that service's static
    // connection info (e.g. CalDAV URI for Calendar) — replaces today's services/*.rs match arms.
    // No token needed: this is provider/service configuration, not per-account data.
    async fn get_service_config(&self, service: &str) -> Result<HashMap<String, String>>;
}
```

(As implemented, both methods return a plain `HashMap<String, String>` rather than dedicated
dict-struct types — simpler to extend per-service without redefining the D-Bus signature.)

Explicitly *not* in this interface: token exchange/refresh (stays generic OAuth2 in `accounts-daemon`), and storage (stays in `accounts-daemon`'s `CredentialStorage`). Providers are stateless request/response services — they receive a token, do one HTTPS call, return normalized data, and can be killed at any time.

D-Bus service activation (`BusName=` in a `.service` file, same pattern as `cosmic-accounts.service`) means providers don't need to be always-running — the bus activates them on first call and they can idle-exit.

### 4. `accounts-daemon` changes

- **`ProviderRegistry`** (new): scans manifest directories at startup, builds `HashMap<String, ProviderManifest>`. Replaces `Provider::services()`/`file_name()` and the enum itself.
- **`auth.rs`**: `AuthManager` reads `auth_url`/`token_url`/`scopes` from the manifest instead of a hardcoded match, and calls the provider's `Provider1::get_user_info` over D-Bus instead of an in-process HTTP request + manual JSON mapping.
- **`services/mod.rs`**: `ServiceFactory` calls `get_service_config` on the relevant provider process instead of matching in `services/calendar.rs`. `CalendarService`'s D-Bus object (consumer-facing) is unchanged — only the source of its `uri` property moves to a provider call.
- **`storage.rs`**: unchanged. Providers never see `CredentialStorage`.

### 5. Google and Microsoft as ordinary providers

`accounts-provider-google` and `accounts-provider-microsoft` ship as their own workspace crates, each a thin zbus service implementing `Provider1` — today's `auth.rs` match-arm bodies extracted directly into two small standalone binaries, with matching manifest files. Nothing about them is special-cased in `accounts-daemon`.

### 6. Third-party providers

A third party ships a binary, a `.service` D-Bus activation file, and a manifest TOML, installed via their own package. No PR to this repo, no recompilation of `accounts-daemon`. `accounts-daemon` picks it up on manifest scan (startup, or a directory watch if hot-reload is wanted later).

## Security notes

- Manifest directories should be root-writable-only in packaged installs (`/usr/share/...`), so a compromised user-session process can't register a rogue provider that gets auto-trusted. A user-local override dir (`$XDG_DATA_HOME/accounts/providers`) can exist for development/sideloading, clearly documented as "you're trusting this code with a live OAuth access token."
- `accounts-daemon` only ever hands a provider a **short-lived access token** for the specific call being made, not the refresh token, not the credential store. A malicious/buggy provider can at worst exfiltrate a token that's already valid for a limited scope/time window, not compromise the whole account.
- D-Bus policy should restrict who can own/call `dev.edfloreshz.Accounts.Provider.*` names, matching how `cosmic-accounts.service` is already policy-constrained.

## Build order

1. ~~Manifest schema (`accounts` crate) + `ProviderRegistry` (`accounts-daemon`), loading `google`/`microsoft` manifests.~~ **Done** — `src/registry.rs`.
2. ~~`list_providers()` on the `Accounts` D-Bus interface; switch `accounts-ui`'s provider picker to it.~~ **Done** — `src/proxy.rs`, `accounts-daemon/src/account.rs`, `accounts-ui/src/app.rs`.
3. ~~`Provider1` D-Bus interface definition.~~ **Done** — `src/proxy.rs`.
4. ~~`accounts-provider-google` / `accounts-provider-microsoft` crates implementing `Provider1`.~~ **Done.**
5. ~~Wire `auth.rs` and `services/mod.rs` to call providers over D-Bus; delete the old match arms.~~ **Done** — `accounts-daemon/src/auth.rs`, `accounts-daemon/src/services/calendar.rs`.
6. ~~Delete the `Provider` enum's compiled-in variants entirely.~~ **Done** — `Provider` is now `pub type Provider = String` in `src/models/provider.rs`. `accounts-daemon/data/providers/*.toml` were rewritten to the new manifest schema rather than deleted, since that directory doubles as `ProviderRegistry`'s repo-relative dev fallback (see `search_dirs()`), which lets `cargo run` work without an installed manifest.
7. Document the manifest format + `Provider1` interface for third-party authors — see below.

**Update:** a follow-up pass migrated `mail.rs` and `todo.rs` to the same `Provider1`-backed pattern as `calendar.rs` — see `ARCHITECTURE.md`. `contacts.rs` remains commented out and still references the old `Provider` enum; it wasn't in scope for that pass.

## Writing a third-party provider

1. Write a manifest (`id`, `name`, `dbus_name`, `services`, `[oauth]` block) — see the example in §1. Install it to `$XDG_DATA_HOME/accounts/providers/<id>.toml` (or `/usr/share/accounts/providers/` for a system package).
2. Implement the `Provider1` D-Bus interface (`accounts::proxy::Provider1Proxy` documents the client side; implement the server side directly with `zbus::interface`, following `accounts-provider-google/src/main.rs` as a reference — it's under 100 lines).
   - `get_user_info(access_token) -> HashMap<String, String>`: keys `display_name`, `username`, optionally `email`.
   - `get_service_config(service) -> HashMap<String, String>`, per supported service:
     - `"calendar"` / `"todo"`: keys `uri`, `accept_ssl_errors`.
     - `"email"`: keys `imap_host`, `imap_supported`, `imap_use_ssl`, `imap_use_tls`, `imap_accept_ssl_errors`, `smtp_host`, `smtp_supported`, `smtp_use_auth`, `smtp_use_ssl`, `smtp_use_tls`, `smtp_accept_ssl_errors`, `smtp_auth_login`, `smtp_auth_plain`, `smtp_auth_xoauth2` (all string-typed; booleans as `"true"`/`"false"`). Identity fields (email address, display name, IMAP/SMTP username) are answered by the daemon from the account itself, not the provider.
   - A provider only needs to implement the services it declares in its manifest — return an error for anything else, as the reference providers do.
3. Register the provider's well-known D-Bus name (matching the manifest's `dbus_name`) — ship a `.service` file (see `accounts-provider-google/data/*.service`) for D-Bus activation, or run the process persistently.
4. That's it — no changes to `accounts`, `accounts-daemon`, or `accounts-ui`. The provider appears in `accounts-ui`'s Add Account grid via `list_providers()` as soon as the manifest is in a searched directory.

## Verification

- `cargo build --workspace` — all five crates compile cleanly with zero errors.
- Manual test: run `accounts-daemon`, `accounts-provider-google`, and `accounts-provider-microsoft` on the session bus, call `list_providers()` and confirm both show up with their declared services without either provider process needing to be involved. Then exercise the add-account flow with real OAuth client credentials to confirm `get_user_info`/`get_service_config` round-trip correctly.
