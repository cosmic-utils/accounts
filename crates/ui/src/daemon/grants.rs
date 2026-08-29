//! Layer-2 authorization: the per-`(account, service, caller)` consent grant
//! table. This is private daemon state, never exposed on the bus. It stores no
//! secrets, but it is the access-control list *for* the secrets, so the file is
//! created `0600` under `$XDG_DATA_HOME/dev.edfloreshz.Accounts/`.
//!
//! See `grants.md` in the protocol spec for the model.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::daemon::{Error, Result};

/// A stored consent decision for one `(account, service, caller)` triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Deny => "deny",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "allow" => Some(Decision::Allow),
            "deny" => Some(Decision::Deny),
            _ => None,
        }
    }
}

/// Canonicalises a service name from the bus (the spec uses `mail`/`tasks`,
/// this codebase's `Service` enum still uses `Email`/`Todo`) to the lowercase
/// spec spelling stored in the grant table. Returns `None` for anything that
/// isn't a real service.
pub fn normalize_service(value: &str) -> Option<&'static str> {
    match value.to_lowercase().as_str() {
        "mail" | "email" => Some("mail"),
        "calendar" => Some("calendar"),
        "contacts" => Some("contacts"),
        "tasks" | "todo" => Some("tasks"),
        _ => None,
    }
}

fn grants_db_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            home.join(".local/share")
        });
    base.join("dev.edfloreshz.Accounts").join("grants.db")
}

#[derive(Clone)]
pub struct GrantStore {
    conn: Arc<Mutex<Connection>>,
}

impl GrantStore {
    pub async fn open() -> Result<Self> {
        let path = grants_db_path();
        let path_for_blocking = path.clone();
        let conn = tokio::task::spawn_blocking(move || -> Result<Connection> {
            if let Some(dir) = path_for_blocking.parent() {
                std::fs::create_dir_all(dir)?;
                std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
            }
            let conn = Connection::open(&path_for_blocking)
                .map_err(|e| Error::StorageError(format!("open grants.db: {e}")))?;
            std::fs::set_permissions(&path_for_blocking, std::fs::Permissions::from_mode(0o600))?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS grants (
                     account_id      TEXT NOT NULL,
                     service         TEXT NOT NULL,
                     caller_identity TEXT NOT NULL,
                     decision        TEXT NOT NULL,
                     granted_at      INTEGER NOT NULL,
                     PRIMARY KEY (account_id, service, caller_identity)
                 );",
            )
            .map_err(|e| Error::StorageError(format!("init grants.db: {e}")))?;
            Ok(conn)
        })
        .await
        .map_err(|e| Error::StorageError(format!("grants.db task: {e}")))??;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn lookup(
        &self,
        account_id: &Uuid,
        service: &str,
        caller_identity: &str,
    ) -> Result<Option<Decision>> {
        let conn = self.conn.clone();
        let account_id = account_id.to_string();
        let service = service.to_string();
        let caller_identity = caller_identity.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<Decision>> {
            let conn = conn.blocking_lock();
            let mut stmt = conn
                .prepare(
                    "SELECT decision FROM grants
                     WHERE account_id = ?1 AND service = ?2 AND caller_identity = ?3",
                )
                .map_err(|e| Error::StorageError(e.to_string()))?;
            let decision: Option<String> = stmt
                .query_row((&account_id, &service, &caller_identity), |row| row.get(0))
                .ok();
            Ok(decision.as_deref().and_then(Decision::parse))
        })
        .await
        .map_err(|e| Error::StorageError(e.to_string()))?
    }

    pub async fn put(
        &self,
        account_id: &Uuid,
        service: &str,
        caller_identity: &str,
        decision: Decision,
    ) -> Result<()> {
        let conn = self.conn.clone();
        let account_id = account_id.to_string();
        let service = service.to_string();
        let caller_identity = caller_identity.to_string();
        let now = chrono::Utc::now().timestamp();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.blocking_lock();
            conn.execute(
                "INSERT INTO grants (account_id, service, caller_identity, decision, granted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (account_id, service, caller_identity)
                 DO UPDATE SET decision = excluded.decision, granted_at = excluded.granted_at",
                (
                    &account_id,
                    &service,
                    &caller_identity,
                    decision.as_str(),
                    now,
                ),
            )
            .map_err(|e| Error::StorageError(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::StorageError(e.to_string()))?
    }

    /// Returns `(service, caller_identity, decision)` rows for one account.
    pub async fn list(&self, account_id: &Uuid) -> Result<Vec<(String, String, String)>> {
        let conn = self.conn.clone();
        let account_id = account_id.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<(String, String, String)>> {
            let conn = conn.blocking_lock();
            let mut stmt = conn
                .prepare(
                    "SELECT service, caller_identity, decision FROM grants
                     WHERE account_id = ?1 ORDER BY service, caller_identity",
                )
                .map_err(|e| Error::StorageError(e.to_string()))?;
            let rows = stmt
                .query_map((&account_id,), |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
                .map_err(|e| Error::StorageError(e.to_string()))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| Error::StorageError(e.to_string()))?;
            Ok(rows)
        })
        .await
        .map_err(|e| Error::StorageError(e.to_string()))?
    }

    pub async fn revoke(
        &self,
        account_id: &Uuid,
        service: &str,
        caller_identity: &str,
    ) -> Result<()> {
        let conn = self.conn.clone();
        let account_id = account_id.to_string();
        let service = service.to_string();
        let caller_identity = caller_identity.to_string();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.blocking_lock();
            conn.execute(
                "DELETE FROM grants
                 WHERE account_id = ?1 AND service = ?2 AND caller_identity = ?3",
                (&account_id, &service, &caller_identity),
            )
            .map_err(|e| Error::StorageError(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::StorageError(e.to_string()))?
    }

    /// Drops every grant for an account, e.g. when it is removed.
    pub async fn clear_account(&self, account_id: &Uuid) -> Result<()> {
        let conn = self.conn.clone();
        let account_id = account_id.to_string();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.blocking_lock();
            conn.execute("DELETE FROM grants WHERE account_id = ?1", (&account_id,))
                .map_err(|e| Error::StorageError(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::StorageError(e.to_string()))?
    }
}
