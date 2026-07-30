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
    async fn get_user_info(&self, access_token: &str) -> Result<DbusUserInfo>;

    // Given a fresh access token and a Service, return that service's connection info
    // (e.g. CalDAV URI for Calendar) — replaces today's services/*.rs match arms
    async fn get_service_config(&self, access_token: &str, service: DbusService) -> Result<ServiceConfig>;
}
```

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

1. Manifest schema (`accounts` crate) + `ProviderRegistry` (`accounts-daemon`), loading `google`/`microsoft` manifests.
2. `list_providers()` on the `Accounts` D-Bus interface; switch `accounts-ui`'s provider picker to it. **Capability advertisement is done here.**
3. `Provider1` D-Bus interface definition.
4. `accounts-provider-google` / `accounts-provider-microsoft` crates implementing `Provider1`.
5. Wire `auth.rs` and `services/mod.rs` to call providers over D-Bus; delete the old match arms.
6. Delete the `Provider` enum's compiled-in variants entirely, along with `accounts-daemon/data/providers/*.toml` (replaced by the XDG manifests).
7. Document the manifest format + `Provider1` interface for third-party authors.

## Verification

- Unit tests for manifest parsing (`ProviderRegistry` loading from a temp XDG dir).
- Integration test: spin up a fake `Provider1` D-Bus service (test double), run the full add-account flow against it, assert `accounts-daemon` never calls out to a real network endpoint for provider-specific data.
- Manual test: confirm Google and Microsoft account flows work end-to-end via `accounts-ui` after the rewrite.
