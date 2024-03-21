#[allow(warnings)]
mod bindings;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use bindings::exports::{
    fermyon::{
        spin::{key_value, llm},
        spin_virt::http_handler,
    },
    wasi::http::outgoing_handler,
};
use bindings::wasi::http::types;

struct Component;

impl key_value::Guest for Component {
    type Store = KeyValueStore;
}

#[derive(Debug, Default, Clone)]
struct KeyValueStore {
    inner: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl key_value::GuestStore for KeyValueStore {
    fn open(label: String) -> Result<key_value::Store, key_value::Error> {
        static STORES: std::sync::OnceLock<Mutex<HashMap<String, KeyValueStore>>> =
            std::sync::OnceLock::new();
        let mut stores = STORES.get_or_init(|| Default::default()).lock().unwrap();
        let key_value = stores.entry(label).or_default();
        Ok(key_value::Store::new(key_value.clone()))
    }

    fn get(&self, key: String) -> Result<Option<Vec<u8>>, key_value::Error> {
        Ok(self.inner.read().unwrap().get(&key).cloned())
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), key_value::Error> {
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
        let response = RESPONSES
            .get_or_init(|| Default::default())
            .lock()
            .unwrap()
            .remove(&url);
        match response {
            Some(r) => Ok(r),
            None => Err(outgoing_handler::ErrorCode::InternalError(Some(format!(
                "{url}"
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

bindings::export!(Component with_types_in bindings);
