use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use serde::Deserialize;

use crate::models::{DbusProviderInfo, Service};

/// Static OAuth2 endpoint/scope configuration for a provider, declared in its manifest.
/// This never requires the provider's own process to be running to be read.
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthManifest {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    /// Extra authorize-URL query parameters this provider needs (e.g. Google's
    /// `access_type=offline` to receive a refresh token). Kept generic so no
    /// provider-specific behavior needs to live in the daemon.
    #[serde(default)]
    pub extra_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderManifestInfo {
    pub id: String,
    pub name: String,
    /// Well-known D-Bus name the provider process registers, e.g.
    /// `dev.edfloreshz.Accounts.Provider.Google`.
    pub dbus_name: String,
    /// Path to the provider's binary. Informational today (the daemon relies on
    /// D-Bus service activation to start it); kept so packaging tooling and
    /// D-Bus `.service` files have a single source of truth for it.
    #[serde(default)]
    pub exec: String,
    pub services: Vec<String>,
}

/// A provider manifest: everything the daemon needs to know about a provider
/// without running its process. Mirrors how `.desktop` files register applets.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderManifest {
    pub provider: ProviderManifestInfo,
    pub oauth: OAuthManifest,
}

impl ProviderManifest {
    /// Services this provider supports, all disabled by default — matches the
    /// shape `Account.services` expects.
    pub fn default_services(&self) -> BTreeMap<Service, bool> {
        self.provider
            .services
            .iter()
            .filter_map(|s| Service::from_str(s.clone()))
            .map(|s| (s, false))
            .collect()
    }

    pub fn info(&self) -> DbusProviderInfo {
        DbusProviderInfo {
            id: self.provider.id.clone(),
            name: self.provider.name.clone(),
            services: self.provider.services.clone(),
        }
    }
}

/// Loads provider manifests from disk and answers "which providers exist / which
/// services do they support" without needing any provider process to be reachable.
#[derive(Debug, Clone, Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, ProviderManifest>,
}

impl ProviderRegistry {
    pub fn load(dirs: &[PathBuf]) -> Self {
        let mut providers = HashMap::new();

        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                    continue;
                }

                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };

                match toml::from_str::<ProviderManifest>(&content) {
                    Ok(manifest) => {
                        providers.insert(manifest.provider.id.clone(), manifest);
                    }
                    Err(err) => {
                        tracing::error!(
                            "Failed to parse provider manifest {}: {}",
                            path.display(),
                            err
                        );
                    }
                }
            }
        }

        Self { providers }
    }

    /// XDG data directories searched for `accounts/providers/*.toml` manifests,
    /// highest-precedence first, plus a repo-relative dev fallback so `cargo run`
    /// works without installing anything.
    pub fn search_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(data_home).join("accounts/providers"));
        } else if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share/accounts/providers"));
        }

        if let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS") {
            for dir in data_dirs.split(':').filter(|d| !d.is_empty()) {
                dirs.push(PathBuf::from(dir).join("accounts/providers"));
            }
        } else {
            dirs.push(PathBuf::from("/usr/local/share/accounts/providers"));
            dirs.push(PathBuf::from("/usr/share/accounts/providers"));
        }

        dirs.push(PathBuf::from("accounts-daemon/data/providers"));

        dirs
    }

    pub fn load_default() -> Self {
        Self::load(&Self::search_dirs())
    }

    pub fn get(&self, id: &str) -> Option<&ProviderManifest> {
        self.providers.get(id)
    }

    pub fn list(&self) -> Vec<&ProviderManifest> {
        self.providers.values().collect()
    }

    pub fn list_infos(&self) -> Vec<DbusProviderInfo> {
        self.providers
            .values()
            .map(ProviderManifest::info)
            .collect()
    }
}
