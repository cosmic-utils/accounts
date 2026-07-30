use std::path::PathBuf;

use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

/// Capability advertisement: what a provider is called and which services it supports.
/// Served straight from the manifest registry, independent of whether the provider's
/// own D-Bus service is currently running.
#[derive(Debug, Clone, PartialEq, DeserializeDict, SerializeDict, Type)]
#[zvariant(signature = "dict")]
pub struct DbusProviderInfo {
    pub id: String,
    pub name: String,
    pub services: Vec<String>,
    /// A remote URL, an absolute path to an icon file (`.svg`/`.png`/...), or a
    /// freedesktop icon-theme name to resolve, e.g. `network-server-symbolic`.
    /// `None` if the provider's manifest doesn't declare one. Use [`Self::icon_source`]
    /// to classify which of the three this is rather than re-deriving it per consumer.
    pub icon: Option<String>,
}

/// The three shapes a provider's `icon` string can take. Consumers still own how to
/// actually turn each variant into pixels (that's toolkit-specific — an icon-theme
/// lookup or image decode looks different in every UI framework) but shouldn't need
/// to re-derive *which* of the three they're looking at.
#[derive(Debug, Clone, PartialEq)]
pub enum IconSource {
    /// Fetch over HTTP(S); the URL is exactly the manifest's `icon` value.
    Url(String),
    /// Load directly from this absolute path.
    Path(PathBuf),
    /// Resolve by name through the platform's icon theme.
    ThemeName(String),
}

impl DbusProviderInfo {
    pub fn icon_source(&self) -> Option<IconSource> {
        let icon = self.icon.as_ref()?;
        if icon.starts_with("http://") || icon.starts_with("https://") {
            Some(IconSource::Url(icon.clone()))
        } else if icon.starts_with('/') {
            Some(IconSource::Path(PathBuf::from(icon)))
        } else {
            Some(IconSource::ThemeName(icon.clone()))
        }
    }
}
