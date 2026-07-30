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
}
