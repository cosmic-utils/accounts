use std::path::PathBuf;

use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

#[derive(Debug, Clone, PartialEq, DeserializeDict, SerializeDict, Type)]
#[zvariant(signature = "dict")]
pub struct DbusProviderInfo {
    pub id: String,
    pub name: String,
    pub services: Vec<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IconSource {
    Url(String),
    Path(PathBuf),
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
