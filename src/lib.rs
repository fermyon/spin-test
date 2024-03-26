#[allow(warnings)]
mod bindings;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, RwLock},
};

use anyhow::Context;
use bindings::exports::{
    fermyon::{
        spin::{key_value, llm},
        spin_test_virt::{http_handler, key_value_calls},
    },
    wasi::http::outgoing_handler,
};
use bindings::wasi::http::types;

struct Component;

impl key_value::Guest for Component {
    type Store = KeyValueStore;
}

#[derive(Debug, Clone)]
struct KeyValueStore {
    label: String,
    inner: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl KeyValueStore {
    fn new(label: String) -> Self {
        Self {
            label,
            inner: Default::default(),
        }
    }
}

impl key_value::GuestStore for KeyValueStore {
    fn open(label: String) -> Result<key_value::Store, key_value::Error> {
        static STORES: std::sync::OnceLock<Mutex<HashMap<String, KeyValueStore>>> =
            std::sync::OnceLock::new();
        let mut stores = STORES.get_or_init(|| Default::default()).lock().unwrap();
        let key_value = stores
            .entry(label.clone())
            .or_insert_with(|| KeyValueStore::new(label));
        Ok(key_value::Store::new(key_value.clone()))
    }

    fn get(&self, key: String) -> Result<Option<Vec<u8>>, key_value::Error> {
        GET_CALLS
            .get_or_init(|| Default::default())
            .write()
            .unwrap()
            .entry(self.label.clone())
            .or_default()
            .push(key_value_calls::GetCall { key: key.clone() });
        Ok(self.inner.read().unwrap().get(&key).cloned())
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), key_value::Error> {
        SET_CALLS
            .get_or_init(|| Default::default())
            .write()
            .unwrap()
            .entry(self.label.clone())
            .or_default()
            .push(key_value_calls::SetCall {
                key: key.clone(),
                value: value.clone(),
            });
        self.inner.write().unwrap().insert(key, value);
        Ok(())
    }

    fn delete(&self, key: String) -> Result<(), key_value::Error> {
        let _ = self.inner.write().unwrap().remove(&key);
        todo!()
    }

    fn exists(&self, key: String) -> Result<bool, key_value::Error> {
        Ok(self.inner.read().unwrap().contains_key(&key))
    }

    fn get_keys(&self) -> Result<Vec<String>, key_value::Error> {
        Ok(self.inner.read().unwrap().keys().cloned().collect())
    }
}

impl llm::Guest for Component {
    fn infer(
        model: llm::InferencingModel,
        prompt: String,
        params: Option<llm::InferencingParams>,
    ) -> Result<llm::InferencingResult, llm::Error> {
        let _ = (model, prompt, params);
        Ok(llm::InferencingResult {
            text: "Hello, world!".to_string(),
            usage: llm::InferencingUsage {
                prompt_token_count: 0,
                generated_token_count: 0,
            },
        })
    }

    fn generate_embeddings(
        model: llm::EmbeddingModel,
        text: Vec<String>,
    ) -> Result<llm::EmbeddingsResult, llm::Error> {
        let _ = (model, text);
        Ok(llm::EmbeddingsResult {
            embeddings: vec![],
            usage: llm::EmbeddingsUsage {
                prompt_token_count: 0,
            },
        })
    }
}

static RESPONSES: std::sync::OnceLock<
    Mutex<HashMap<String, outgoing_handler::FutureIncomingResponse>>,
> = std::sync::OnceLock::new();

impl outgoing_handler::Guest for Component {
    fn handle(
        request: outgoing_handler::OutgoingRequest,
        _options: Option<outgoing_handler::RequestOptions>,
    ) -> Result<outgoing_handler::FutureIncomingResponse, outgoing_handler::ErrorCode> {
        let url = format!(
            "{scheme}://{authority}{path_and_query}",
            scheme = match request.scheme() {
                Some(types::Scheme::Http) => "http",
                Some(types::Scheme::Https) | None => "https",
                Some(types::Scheme::Other(ref s)) => s,
            },
            authority = request.authority().expect("TODO: handle"),
            path_and_query = request
                .path_with_query()
                .filter(|p| p != "/")
                .unwrap_or_default()
        );
        let url_allowed = AppManifest::allows_url(&url)
            .map_err(|e| outgoing_handler::ErrorCode::InternalError(Some(format!("{e}"))))?;
        if !url_allowed {
            return Err(outgoing_handler::ErrorCode::HttpRequestDenied);
        }
        let response = RESPONSES
            .get_or_init(|| Default::default())
            .lock()
            .unwrap()
            .remove(&url);
        match response {
            Some(r) => Ok(r),
            None => Err(outgoing_handler::ErrorCode::InternalError(Some(format!(
                "unrecognized url: {url}"
            )))),
        }
    }
}

impl http_handler::Guest for Component {
    fn set_response(url: String, response: http_handler::FutureIncomingResponse) {
        RESPONSES
            .get_or_init(|| Default::default())
            .lock()
            .unwrap()
            .insert(url, response);
    }
}

static GET_CALLS: OnceLock<RwLock<HashMap<String, Vec<key_value_calls::GetCall>>>> =
    OnceLock::new();
static SET_CALLS: OnceLock<RwLock<HashMap<String, Vec<key_value_calls::SetCall>>>> =
    OnceLock::new();

impl key_value_calls::Guest for Component {
    fn get() -> Vec<(String, Vec<key_value_calls::GetCall>)> {
        GET_CALLS
            .get_or_init(Default::default)
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn set() -> Vec<(String, Vec<key_value_calls::SetCall>)> {
        SET_CALLS
            .get_or_init(Default::default)
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn reset() {
        SET_CALLS
            .get_or_init(Default::default)
            .write()
            .unwrap()
            .clear();
        GET_CALLS
            .get_or_init(Default::default)
            .write()
            .unwrap()
            .clear();
    }
}

static MANIFEST: OnceLock<spin_manifest::schema::v2::AppManifest> = OnceLock::new();
struct AppManifest;

impl AppManifest {
    fn allows_url(url: &str) -> anyhow::Result<bool> {
        let mut manifest = Self::get()?;
        spin_manifest::normalize::normalize_manifest(&mut manifest);
        let id: spin_serde::KebabId = COMPONENT_ID
            .read()
            .unwrap()
            .clone()
            .try_into()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let component = manifest
            .components
            .get(&id)
            .with_context(|| format!("component '{id}'not found"))?;
        let allowed_outbound_hosts = component.normalized_allowed_outbound_hosts()?;
        let resolver = spin_expressions::PreparedResolver::default();
        let allowed_hosts = spin_outbound_networking::AllowedHostsConfig::parse(
            &allowed_outbound_hosts,
            &resolver,
        )?;
        let url = spin_outbound_networking::OutboundUrl::parse(url, "https")?;
        Ok(allowed_hosts.allows(&url))
    }

    fn get() -> anyhow::Result<spin_manifest::schema::v2::AppManifest> {
        if let Some(m) = MANIFEST.get() {
            return Ok(m.clone());
        }
        let Ok(deserialize) = toml::from_str(&bindings::get_manifest()) else {
            anyhow::bail!("failed to deserialize manifest");
        };
        Ok(MANIFEST.get_or_init(|| deserialize).clone())
    }
}

static COMPONENT_ID: RwLock<String> = RwLock::new(String::new());
impl bindings::Guest for Component {
    fn set_component_id(component_id: String) {
        *COMPONENT_ID.write().unwrap() = component_id;
    }
}

bindings::export!(Component with_types_in bindings);
