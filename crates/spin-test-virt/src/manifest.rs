use std::sync::{OnceLock, RwLock};

/// The manifest for the current Spin app.
pub struct AppManifest;

impl AppManifest {
    /// Returns the allowed hosts configuration for the current component.
    pub fn allowed_hosts() -> anyhow::Result<spin_outbound_networking::AllowedHostsConfig> {
        let allowed_outbound_hosts = Self::get_component()
            .expect("internal error: component id not yet set")
            .normalized_allowed_outbound_hosts()?;
        let resolver = spin_expressions::PreparedResolver::default();
        spin_outbound_networking::AllowedHostsConfig::parse(&allowed_outbound_hosts, &resolver)
    }

    /// Returns whether the given URL is allowed by the manifest.
    pub fn allows_url(url: &str, scheme: &str) -> anyhow::Result<bool> {
        let allowed_hosts = Self::allowed_hosts()?;
        let url = spin_outbound_networking::OutboundUrl::parse(url, scheme)?;
        Ok(allowed_hosts.allows(&url))
    }

    /// Returns the manifest for the current component.
    pub fn get() -> spin_manifest::schema::v2::AppManifest {
        static MANIFEST: OnceLock<spin_manifest::schema::v2::AppManifest> = OnceLock::new();
        MANIFEST
            .get_or_init(|| {
                let mut manifest =
                    toml::from_str(&crate::bindings::get_manifest()).unwrap_or_else(|_| {
                        panic!("internal error: manifest was malformed");
                    });

                spin_manifest::normalize::normalize_manifest(&mut manifest);
                manifest
            })
            .clone()
    }

    /// Gets the current component from the manifest.
    ///
    /// Returns `None` if the component id has not been set.
    pub fn get_component() -> Option<spin_manifest::schema::v2::Component> {
        Some(
            Self::get()
                .components
                .remove(&Self::get_component_id()?)
                .expect("internal error: component not found in manifest"),
        )
    }

    /// Gets the ID of the current component.
    ///
    /// Returns `None` if the component id has not been set.
    pub fn get_component_id() -> Option<spin_serde::KebabId> {
        Some(
            COMPONENT_ID
                .read()
                .unwrap()
                .clone()?
                .try_into()
                .expect("internal error: component ID is not kebab-case"),
        )
    }
}

static COMPONENT_ID: RwLock<Option<String>> = RwLock::new(None);
impl crate::bindings::Guest for crate::Component {
    fn set_component_id(component_id: String) {
        *COMPONENT_ID.write().unwrap() = Some(component_id);
    }
}
