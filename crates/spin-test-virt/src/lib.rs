#[allow(warnings)]
mod bindings;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, RwLock},
};

use bindings::exports::{
    fermyon::{
        spin::{key_value, llm, mysql, postgres, redis, sqlite, variables},
        spin_test_virt::{http_handler, key_value_calls},
    },
    wasi::http::outgoing_handler,
};
use bindings::{exports::fermyon::spin::mqtt, wasi::http::types};

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

impl redis::Guest for Component {
    type Connection = RedisConnection;
}

struct RedisConnection;

impl redis::GuestConnection for RedisConnection {
    fn open(address: String) -> Result<redis::Connection, redis::Error> {
        let _ = address;
        todo!()
    }

    fn publish(&self, channel: String, payload: redis::Payload) -> Result<(), redis::Error> {
        let _ = (channel, payload);
        todo!()
    }

    fn get(&self, key: String) -> Result<Option<redis::Payload>, redis::Error> {
        let _ = key;
        todo!()
    }

    fn set(&self, key: String, value: redis::Payload) -> Result<(), redis::Error> {
        let _ = (key, value);
        todo!()
    }

    fn incr(&self, key: String) -> Result<i64, redis::Error> {
        let _ = key;
        todo!()
    }

    fn del(&self, keys: Vec<String>) -> Result<u32, redis::Error> {
        let _ = keys;
        todo!()
    }

    fn sadd(&self, key: String, values: Vec<String>) -> Result<u32, redis::Error> {
        let _ = (key, values);
        todo!()
    }

    fn smembers(&self, key: String) -> Result<Vec<String>, redis::Error> {
        let _ = key;
        todo!()
    }

    fn srem(&self, key: String, values: Vec<String>) -> Result<u32, redis::Error> {
        let _ = (key, values);
        todo!()
    }

    fn execute(
        &self,
        command: String,
        arguments: Vec<redis::RedisParameter>,
    ) -> Result<Vec<redis::RedisResult>, redis::Error> {
        let _ = (command, arguments);
        todo!()
    }
}

impl sqlite::Guest for Component {
    type Connection = SqliteConnection;
}

struct SqliteConnection;

impl sqlite::GuestConnection for SqliteConnection {
    fn open(database: String) -> Result<sqlite::Connection, sqlite::Error> {
        let _ = database;
        todo!()
    }

    fn execute(
        &self,
        statement: String,
        parameters: Vec<sqlite::Value>,
    ) -> Result<sqlite::QueryResult, sqlite::Error> {
        let _ = (statement, parameters);
        todo!()
    }
}

impl mysql::Guest for Component {
    type Connection = MySqlConnection;
}

struct MySqlConnection;

impl mysql::GuestConnection for MySqlConnection {
    fn open(address: String) -> Result<mysql::Connection, mysql::Error> {
        let _ = address;
        todo!()
    }

    fn query(
        &self,
        statement: String,
        params: Vec<mysql::ParameterValue>,
    ) -> Result<mysql::RowSet, mysql::Error> {
        let _ = (statement, params);
        todo!()
    }

    fn execute(
        &self,
        statement: String,
        params: Vec<mysql::ParameterValue>,
    ) -> Result<(), mysql::Error> {
        let _ = (statement, params);
        todo!()
    }
}

impl postgres::Guest for Component {
    type Connection = PostgresConnection;
}

struct PostgresConnection;

impl postgres::GuestConnection for PostgresConnection {
    fn open(address: String) -> Result<postgres::Connection, postgres::Error> {
        let _ = address;
        todo!()
    }

    fn query(
        &self,
        statement: String,
        params: Vec<postgres::ParameterValue>,
    ) -> Result<postgres::RowSet, postgres::Error> {
        let _ = (statement, params);
        todo!()
    }

    fn execute(
        &self,
        statement: String,
        params: Vec<postgres::ParameterValue>,
    ) -> Result<u64, postgres::Error> {
        let _ = (statement, params);
        todo!()
    }
}

impl mqtt::Guest for Component {
    type Connection = MqttConnection;
}

struct MqttConnection;

impl mqtt::GuestConnection for MqttConnection {
    fn open(
        address: String,
        username: String,
        password: String,
        keep_alive_interval_in_secs: u64,
    ) -> Result<mqtt::Connection, mqtt::Error> {
        let _ = (address, username, password, keep_alive_interval_in_secs);
        todo!()
    }

    fn publish(
        &self,
        topic: String,
        payload: mqtt::Payload,
        qos: mqtt::Qos,
    ) -> Result<(), mqtt::Error> {
        let _ = (topic, payload, qos);
        todo!()
    }
}

impl variables::Guest for Component {
    fn get(name: String) -> Result<String, variables::Error> {
        let app_variables = AppManifest::get()
            .variables
            .into_iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    spin_locked_app::Variable {
                        default: v.default,
                        secret: v.secret,
                    },
                )
            })
            .collect::<Vec<_>>();
        let mut resolver = spin_expressions::Resolver::new(app_variables).unwrap();
        let component_id = AppManifest::get_component_id();
        let component_variables = AppManifest::get_component()
            .variables
            .into_iter()
            .map(|(k, v)| (k.to_string(), v));
        resolver
            .add_component_variables(component_id.as_ref(), component_variables)
            .unwrap();
        let key = spin_expressions::Key::new(&name).unwrap();
        resolver
            .resolve(component_id.as_ref(), key)
            .map_err(|_| variables::Error::Undefined(name.to_string()))
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
        let mut manifest = Self::get();
        spin_manifest::normalize::normalize_manifest(&mut manifest);
        let allowed_outbound_hosts = Self::get_component().normalized_allowed_outbound_hosts()?;
        let resolver = spin_expressions::PreparedResolver::default();
        let allowed_hosts = spin_outbound_networking::AllowedHostsConfig::parse(
            &allowed_outbound_hosts,
            &resolver,
        )?;
        let url = spin_outbound_networking::OutboundUrl::parse(url, "https")?;
        Ok(allowed_hosts.allows(&url))
    }

    fn get() -> spin_manifest::schema::v2::AppManifest {
        if let Some(m) = MANIFEST.get() {
            return m.clone();
        }
        MANIFEST
            .get_or_init(|| {
                toml::from_str(&bindings::get_manifest()).unwrap_or_else(|_| {
                    panic!("internal error: manifest was malformed");
                })
            })
            .clone()
    }

    fn get_component() -> spin_manifest::schema::v2::Component {
        Self::get()
            .components
            .remove(&Self::get_component_id())
            .expect("internal error: component not found in manifest")
    }

    fn get_component_id() -> spin_serde::id::Id<'-'> {
        let component_id: spin_serde::KebabId = COMPONENT_ID
            .read()
            .expect("internal error: component ID has not been set")
            .clone()
            .try_into()
            .expect("internal error: component ID is not kebab-case");
        component_id
    }
}

static COMPONENT_ID: RwLock<String> = RwLock::new(String::new());
impl bindings::Guest for Component {
    fn set_component_id(component_id: String) {
        *COMPONENT_ID.write().unwrap() = component_id;
    }
}

bindings::export!(Component with_types_in bindings);
