use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use serde::Deserialize;

use crate::models::{DbusProviderInfo, Service};

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthManifest {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    #[serde(default)]
    pub extra_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderManifestInfo {
    pub id: String,
    pub name: String,
    pub services: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

/// How the daemon learns the account identity after a successful OAuth2 flow:
/// one authenticated `GET` against `url`, then pull these fields out of the JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct UserInfoManifest {
    pub url: String,
    #[serde(default = "default_display_name_field")]
    pub display_name_field: String,
    #[serde(default = "default_email_field")]
    pub email_field: String,
    /// Field to use as the stable username; falls back to `email_field`.
    #[serde(default)]
    pub username_field: Option<String>,
}

fn default_display_name_field() -> String {
    "name".to_string()
}

fn default_email_field() -> String {
    "email".to_string()
}

/// A CalDAV/CardDAV collection URL. `${identity}` in the template is replaced
/// with the account's identity (email or username) at resolution time.
#[derive(Debug, Clone, Deserialize)]
pub struct DavEndpointManifest {
    pub uri_template: String,
}

impl DavEndpointManifest {
    pub fn resolve(&self, identity: &str) -> String {
        self.uri_template.replace("${identity}", identity)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MailEndpointManifest {
    pub imap_host: String,
    #[serde(default = "default_imap_port")]
    pub imap_port: u16,
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
}

fn default_imap_port() -> u16 {
    993
}

fn default_smtp_port() -> u16 {
    587
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EndpointManifest {
    #[serde(default)]
    pub calendar: Option<DavEndpointManifest>,
    #[serde(default)]
    pub contacts: Option<DavEndpointManifest>,
    #[serde(default)]
    pub tasks: Option<DavEndpointManifest>,
    #[serde(default)]
    pub mail: Option<MailEndpointManifest>,
}

/// Present only for providers whose auth flow the daemon can't drive itself
/// (device-code, custom SSO, client certs). Points at a service implementing
/// `dev.edfloreshz.Accounts.ProviderHandler`.
#[derive(Debug, Clone, Deserialize)]
pub struct HandlerManifest {
    pub bus_name: String,
    pub object_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderManifest {
    pub provider: ProviderManifestInfo,
    pub oauth: OAuthManifest,
    #[serde(default)]
    pub userinfo: Option<UserInfoManifest>,
    #[serde(default)]
    pub endpoint: EndpointManifest,
    #[serde(default)]
    pub handler: Option<HandlerManifest>,
}

impl ProviderManifest {
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
            icon: self.provider.icon.clone(),
        }
    }
}

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
                        providers
                            .entry(manifest.provider.id.clone())
                            .or_insert(manifest);
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

        dirs.push(PathBuf::from("crates/ui/data/providers"));

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
