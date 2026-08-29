//! Server-side resolution of a D-Bus caller's stable identity for the layer-2
//! grant table. A self-reported id in method arguments is never trusted — the
//! identity is derived from the bus connection's credentials.
//!
//! Preference order (per `grants.md`):
//! 1. sandbox app id (Flatpak / snap) — this is what the user recognises in a
//!    consent prompt;
//! 2. the resolved absolute path of the process's executable, read fresh from
//!    `/proc/<pid>/exe` every time (pids are recycled, so it is never cached);
//! 3. otherwise unidentifiable — such a caller may be issued a one-time token
//!    after a prompt but never gets a persisted grant.

use std::path::PathBuf;

use zbus::message::Header;

pub struct Caller {
    /// Stable key for the grant table, or `None` if the caller can't be identified.
    pub identity: Option<String>,
    /// Human-readable label for the consent prompt.
    pub display_name: String,
}

const UNIDENTIFIED: &str = "an unidentified application";

pub async fn resolve(connection: &zbus::Connection, header: &Header<'_>) -> Caller {
    match resolve_inner(connection, header).await {
        Some(caller) => caller,
        None => Caller {
            identity: None,
            display_name: UNIDENTIFIED.to_string(),
        },
    }
}

async fn resolve_inner(connection: &zbus::Connection, header: &Header<'_>) -> Option<Caller> {
    let sender = header.sender()?.to_owned();
    let dbus = zbus::fdo::DBusProxy::new(connection).await.ok()?;
    let credentials = dbus
        .get_connection_credentials(zbus::names::BusName::from(sender))
        .await
        .ok()?;
    let pid = credentials.process_id()?;

    if let Some(app_id) = flatpak_app_id(pid).or_else(|| snap_name(pid)) {
        return Some(Caller {
            display_name: app_id.clone(),
            identity: Some(app_id),
        });
    }

    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    let display_name = exe
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| exe.to_string_lossy().into_owned());
    Some(Caller {
        identity: Some(exe.to_string_lossy().into_owned()),
        display_name,
    })
}

/// Reads the Flatpak sandbox app id from `/proc/<pid>/root/.flatpak-info`
/// (`[Application] name=`), which Flatpak writes into every sandboxed process.
fn flatpak_app_id(pid: u32) -> Option<String> {
    let info =
        std::fs::read_to_string(PathBuf::from(format!("/proc/{pid}/root/.flatpak-info"))).ok()?;
    let mut in_application = false;
    for line in info.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_application = line == "[Application]";
        } else if in_application && let Some(name) = line.strip_prefix("name=") {
            let name = name.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Extracts a snap name from the process cgroup (`.../snap.<name>.<app>.service`).
fn snap_name(pid: u32) -> Option<String> {
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    let marker = "snap.";
    let start = cgroup.find(marker)? + marker.len();
    let rest = &cgroup[start..];
    let name: String = rest
        .chars()
        .take_while(|c| *c != '.' && *c != '/')
        .collect();
    (!name.is_empty()).then(|| format!("snap.{name}"))
}
