/// A provider is identified by the `id` declared in its manifest (e.g. `"google"`).
/// Providers are no longer a closed set compiled into this crate — see [`crate::registry`].
pub type Provider = String;
