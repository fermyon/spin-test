mod helper;

#[allow(warnings)]
mod bindings;
mod manifest;
mod wasi;

use std::{
    cell::{LazyCell, RefCell},
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, OnceLock, RwLock},
};

use bindings::exports::fermyon::{
    spin::{self, llm, mqtt, mysql, postgres, redis, sqlite, variables},
    spin_test_virt::{self, key_value as virt_key_value},
};

struct Component;

impl spin::key_value::Guest for Component {
    type Store = KeyValueStore;
}

/// The global collection of key-value stores.
struct Stores;

impl Stores {
    /// Get the global collection of key-value stores.
    ///
    /// The keys are the labels of the stores.
    fn get() -> &'static RwLock<HashMap<String, KeyValueStore>> {
        static STORES: OnceLock<RwLock<HashMap<String, KeyValueStore>>> = OnceLock::new();
        STORES.get_or_init(Default::default)
    }
}

/// An instance of a key-value store.
#[derive(Debug, Clone)]
struct KeyValueStore {
    label: String,
    /// The data stored in the key-value store.
    data: SharedHashMap<String, Vec<u8>>,
    /// The calls made to the key-value store.
    calls: SharedHashMap<String, Vec<virt_key_value::Call>>,
}

type SharedHashMap<K, V> = Arc<RwLock<HashMap<K, V>>>;

impl KeyValueStore {
    /// Create a new key-value store.
    fn new(label: String) -> Self {
        Self {
            label,
            data: Default::default(),
            calls: Default::default(),
        }
    }

    /// Get the value associated with a key.
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.read_data().get(key).cloned()
    }

    /// Set the value associated with a key.
    fn set(&self, key: String, value: Vec<u8>) {
        self.write_data().insert(key.clone(), value.clone());
    }

    /// Delete the value associated with a key.
    fn delete(&self, key: &str) {
        self.write_data().remove(key);
    }

    /// Check if a key exists in the key-value store.
    fn exists(&self, key: &str) -> bool {
        self.read_data().contains_key(key)
    }

    /// Get the keys in the key-value store.
    fn get_keys(&self) -> Vec<String> {
        self.read_data().keys().cloned().collect()
    }

    /// Clear the recorded calls made to the key-value store.
    fn clear_calls(&self) {
        self.calls.write().unwrap().clear()
    }

    fn write_data(&self) -> std::sync::RwLockWriteGuard<'_, HashMap<String, Vec<u8>>> {
        self.data.write().unwrap()
    }

    fn read_data(&self) -> std::sync::RwLockReadGuard<'_, HashMap<String, Vec<u8>>> {
        self.data.read().unwrap()
    }

    fn push_call(&self, call: virt_key_value::Call) {
        self.calls
            .write()
            .unwrap()
            .entry(self.label.clone())
            .or_default()
            .push(call)
    }

    fn read_calls(&self) -> Vec<virt_key_value::Call> {
        self.calls
            .read()
            .unwrap()
            .get(&self.label)
            .cloned()
            .unwrap_or_default()
    }
}

impl spin::key_value::GuestStore for KeyValueStore {
    fn open(label: String) -> Result<spin::key_value::Store, spin::key_value::Error> {
        if let Some(component) = manifest::AppManifest::get_component() {
            // Only allow opening stores that are defined in the manifest.
            // This check should only be done when we have a manifest.
            let store = component
                .key_value_stores
                .into_iter()
                .find(|store| store == &label);
            if store.is_none() {
                return Err(spin::key_value::Error::AccessDenied);
            }
        }

        let mut stores = Stores::get().write().unwrap();
        let key_value = stores
            .entry(label.clone())
            .or_insert_with(|| KeyValueStore::new(label));
        Ok(spin::key_value::Store::new(key_value.clone()))
    }

    fn get(&self, key: String) -> Result<Option<Vec<u8>>, spin::key_value::Error> {
        let result = self.get(&key);
        self.push_call(virt_key_value::Call::Get(key));
        Ok(result)
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), spin::key_value::Error> {
        self.set(key.clone(), value.clone());
        self.push_call(virt_key_value::Call::Set((key, value)));
        Ok(())
    }

    fn delete(&self, key: String) -> Result<(), spin::key_value::Error> {
        self.delete(&key);
        self.push_call(virt_key_value::Call::Delete(key));
        Ok(())
    }

    fn exists(&self, key: String) -> Result<bool, spin::key_value::Error> {
        let result = self.exists(&key);
        self.push_call(virt_key_value::Call::Exists(key));
        Ok(result)
    }

    fn get_keys(&self) -> Result<Vec<String>, spin::key_value::Error> {
        self.push_call(virt_key_value::Call::GetKeys);
        Ok(self.get_keys())
    }
}

impl llm::Guest for Component {
    fn infer(
        model: llm::InferencingModel,
        prompt: String,
        params: Option<llm::InferencingParams>,
    ) -> Result<llm::InferencingResult, llm::Error> {
        check_model(&model)?;
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
        check_model(&model)?;
        let _ = (model, text);
        Ok(llm::EmbeddingsResult {
            embeddings: vec![],
            usage: llm::EmbeddingsUsage {
                prompt_token_count: 0,
            },
        })
    }
}

fn check_model(model: &str) -> Result<(), llm::Error> {
    let model_allowed = manifest::AppManifest::get_component()
        .expect("internal error: component id not yet set")
        .ai_models
        .into_iter()
        .any(|m| m.as_ref() == model);

    if !model_allowed {
        return Err(llm::Error::ModelNotSupported);
    }

    Ok(())
}

impl redis::Guest for Component {
    type Connection = RedisConnection;
}

struct RedisConnection {
    /// The data stored in the Redis store.
    data: SharedHashMap<String, RedisValue>,
}

enum RedisValue {
    Payload(redis::Payload),
    Set(HashSet<String>),
}

impl RedisConnection {
    fn new() -> Self {
        Self {
            data: Default::default(),
        }
    }

    /// Get the redis payload associated with a key.
    ///
    /// Returns `Ok(None)` if the key does not exist and `Err(redis::Error::TypeError)` if the key
    /// exists but is not a payload.
    fn get_payload(&self, key: String) -> Result<Option<redis::Payload>, redis::Error> {
        match self.read_data().get(&key) {
            Some(RedisValue::Payload(p)) => Ok(Some(p.clone())),
            Some(RedisValue::Set(_)) => Err(redis::Error::TypeError),
            None => Ok(None),
        }
    }

    /// Get the set associated with the key.
    fn get_set(&self, key: String) -> Result<HashSet<String>, redis::Error> {
        match self.read_data().get(&key) {
            Some(RedisValue::Set(s)) => Ok(s.clone()),
            Some(RedisValue::Payload(_)) => Err(redis::Error::TypeError),
            None => Ok(Default::default()),
        }
    }

    /// Add the values to the set associated with the key.
    fn add_to_set(&self, key: String, new: Vec<String>) -> Result<usize, redis::Error> {
        match self.write_data().get_mut(&key) {
            Some(RedisValue::Set(s)) => {
                let original_len = s.len();
                s.extend(new);
                Ok(s.len() - original_len)
            }
            Some(RedisValue::Payload(_)) => Err(redis::Error::TypeError),
            None => {
                let set = new.into_iter().collect::<HashSet<_>>();
                let len = set.len();
                self.write_data().insert(key.clone(), RedisValue::Set(set));
                Ok(len)
            }
        }
    }

    /// Remove the values from the set associated with the key.
    fn remove_from_set(&self, key: String, values: Vec<String>) -> Result<usize, redis::Error> {
        match self.write_data().get_mut(&key) {
            Some(RedisValue::Set(s)) => {
                let original_len = s.len();
                s.retain(|v| !values.contains(v));
                Ok(original_len - s.len())
            }
            Some(RedisValue::Payload(_)) => Err(redis::Error::TypeError),
            None => Ok(0),
        }
    }

    /// Set the value associated with a key.
    fn set(&self, key: String, value: redis::Payload) {
        self.write_data().insert(key, RedisValue::Payload(value));
    }

    /// Delete the values associated with the keys.
    ///
    /// Returns the number of keys that were deleted.
    fn del(&self, keys: Vec<String>) -> usize {
        let mut data = self.write_data();
        let original_len = data.len();
        data.retain(|k, _| !keys.contains(k));
        let new_len = data.len();
        original_len - new_len
    }

    fn write_data(&self) -> std::sync::RwLockWriteGuard<'_, HashMap<String, RedisValue>> {
        self.data.write().unwrap()
    }

    fn read_data(&self) -> std::sync::RwLockReadGuard<'_, HashMap<String, RedisValue>> {
        self.data.read().unwrap()
    }
}

impl redis::GuestConnection for RedisConnection {
    fn open(address: String) -> Result<redis::Connection, redis::Error> {
        let url_allowed = manifest::AppManifest::allows_url(&address, "redis")
            .map_err(|_| redis::Error::InvalidAddress)?;
        if !url_allowed {
            return Err(redis::Error::InvalidAddress);
        }
        Ok(redis::Connection::new(RedisConnection::new()))
    }

    fn publish(&self, channel: String, payload: redis::Payload) -> Result<(), redis::Error> {
        let _ = (channel, payload);
        Ok(())
    }

    fn get(&self, key: String) -> Result<Option<redis::Payload>, redis::Error> {
        self.get_payload(key)
    }

    fn set(&self, key: String, value: redis::Payload) -> Result<(), redis::Error> {
        self.set(key, value);
        Ok(())
    }

    fn incr(&self, key: String) -> Result<i64, redis::Error> {
        let value = self
            .get_payload(key)?
            .map(String::from_utf8)
            .transpose()
            .map_err(|_| redis::Error::TypeError)?;
        let result = value
            .map(|v| v.parse::<i64>())
            .transpose()
            .map_err(|_| redis::Error::TypeError)?;
        Ok(result.unwrap_or(0) + 1)
    }

    fn del(&self, keys: Vec<String>) -> Result<u32, redis::Error> {
        Ok(self.del(keys) as u32)
    }

    fn sadd(&self, key: String, values: Vec<String>) -> Result<u32, redis::Error> {
        self.add_to_set(key, values).map(|n| n as u32)
    }

    fn smembers(&self, key: String) -> Result<Vec<String>, redis::Error> {
        self.get_set(key).map(|s| s.into_iter().collect())
    }

    fn srem(&self, key: String, values: Vec<String>) -> Result<u32, redis::Error> {
        self.remove_from_set(key, values).map(|n| n as u32)
    }

    fn execute(
        &self,
        command: String,
        mut arguments: Vec<redis::RedisParameter>,
    ) -> Result<Vec<redis::RedisResult>, redis::Error> {
        let mut get_binary = || match arguments.pop().ok_or(redis::Error::TypeError)? {
            redis::RedisParameter::Int64(_) => Err(redis::Error::TypeError),
            redis::RedisParameter::Binary(b) => Ok(b),
        };
        match command.as_str() {
            "incr" => {
                let key = get_binary()?;
                let key = String::from_utf8(key).map_err(|_| redis::Error::TypeError)?;
                self.incr(key).map(|i| vec![redis::RedisResult::Int64(i)])
            }
            "set" => {
                let value = get_binary()?;
                let key = get_binary()?;
                let key = String::from_utf8(key).map_err(|_| redis::Error::TypeError)?;
                self.set(key, value);
                Ok(vec![])
            }
            "append" => {
                let value = get_binary()?;
                let key = get_binary()?;
                let key = String::from_utf8(key).map_err(|_| redis::Error::TypeError)?;
                let mut current = self.get(key.clone())?.unwrap_or_default();
                current.extend(value);
                self.set(key, current);
                Ok(vec![])
            }
            "get" => {
                let key = get_binary()?;
                let key = String::from_utf8(key).map_err(|_| redis::Error::TypeError)?;
                let value = self.get(key)?;
                Ok(value
                    .map(|v| vec![redis::RedisResult::Binary(v)])
                    .unwrap_or_default())
            }
            _ => {
                // TODO: implement this by getting input from user
                Err(redis::Error::Other(format!(
                    "not able to execute '{command}' command"
                )))
            }
        }
    }
}

impl sqlite::Guest for Component {
    type Connection = SqliteConnection;
}

type ConnectionPool = HashMap<String, Arc<Mutex<rusqlite::Connection>>>;
static SQLITE_CONNECTION_POOL: std::sync::OnceLock<Mutex<ConnectionPool>> =
    std::sync::OnceLock::new();

struct SqliteConnection {
    inner: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteConnection {
    fn new(database: String) -> Result<Self, sqlite::Error> {
        let conn = match SQLITE_CONNECTION_POOL
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .entry(database)
        {
            std::collections::hash_map::Entry::Occupied(c) => c.get().clone(),
            std::collections::hash_map::Entry::Vacant(_) => {
                let conn = rusqlite::Connection::open_in_memory()
                    .map_err(|e| sqlite::Error::Io(e.to_string()))?;
                Arc::new(Mutex::new(conn))
            }
        };
        Ok(Self { inner: conn })
    }

    fn execute(
        &self,
        statement: String,
        parameters: Vec<sqlite::Value>,
    ) -> Result<sqlite::QueryResult, sqlite::Error> {
        let conn = self.inner.lock().unwrap();
        let mut prepared = conn
            .prepare(&statement)
            .map_err(|e| sqlite::Error::Io(e.to_string()))?;
        let columns = prepared
            .column_names()
            .into_iter()
            .map(String::from)
            .collect();
        let mut result = sqlite::QueryResult {
            columns,
            rows: vec![],
        };
        let params = parameters.into_iter().map(|v| match v {
            sqlite::Value::Integer(i) => rusqlite::types::Value::Integer(i),
            sqlite::Value::Real(r) => rusqlite::types::Value::Real(r),
            sqlite::Value::Text(t) => rusqlite::types::Value::Text(t),
            sqlite::Value::Blob(b) => rusqlite::types::Value::Blob(b),
            sqlite::Value::Null => rusqlite::types::Value::Null,
        });
        let rows = prepared
            .query_map(rusqlite::params_from_iter(params), |row| {
                let mut values = Vec::new();
                for i in 0..result.columns.len() {
                    let v = match row.get(i)? {
                        rusqlite::types::Value::Null => sqlite::Value::Null,
                        rusqlite::types::Value::Integer(i) => sqlite::Value::Integer(i),
                        rusqlite::types::Value::Real(f) => sqlite::Value::Real(f),
                        rusqlite::types::Value::Text(s) => sqlite::Value::Text(s),
                        rusqlite::types::Value::Blob(b) => sqlite::Value::Blob(b),
                    };
                    values.push(v);
                }
                Ok(sqlite::RowResult { values })
            })
            .map_err(|e| sqlite::Error::Io(e.to_string()))?;

        for row in rows {
            result
                .rows
                .push(row.map_err(|e| sqlite::Error::Io(e.to_string()))?);
        }
        Ok(result)
    }
}

impl sqlite::GuestConnection for SqliteConnection {
    fn open(database: String) -> Result<sqlite::Connection, sqlite::Error> {
        let component =
            manifest::AppManifest::get_component().expect("component id has not been set");
        let db = component
            .sqlite_databases
            .into_iter()
            .find(|db| db == &database);
        if db.is_none() {
            return Err(sqlite::Error::AccessDenied);
        }
        Ok(sqlite::Connection::new(SqliteConnection::new(database)?))
    }

    fn execute(
        &self,
        statement: String,
        parameters: Vec<sqlite::Value>,
    ) -> Result<sqlite::QueryResult, sqlite::Error> {
        self.execute(statement, parameters)
    }
}

impl spin_test_virt::sqlite::Guest for Component {
    type Connection = SqliteConnection;
}

impl spin_test_virt::sqlite::GuestConnection for SqliteConnection {
    fn open(
        database: String,
    ) -> Result<spin_test_virt::sqlite::Connection, spin_test_virt::sqlite::Error> {
        Ok(spin_test_virt::sqlite::Connection::new(
            SqliteConnection::new(database)?,
        ))
    }

    fn execute(
        &self,
        statement: String,
        parameters: Vec<spin_test_virt::sqlite::Value>,
    ) -> Result<spin_test_virt::sqlite::QueryResult, spin_test_virt::sqlite::Error> {
        self.execute(statement, parameters)
    }
}

impl std::hash::Hash for sqlite::Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            sqlite::Value::Null => 0.hash(state),
            sqlite::Value::Integer(i) => i.hash(state),
            sqlite::Value::Real(f) => f.to_bits().hash(state),
            sqlite::Value::Text(s) => s.hash(state),
            sqlite::Value::Blob(b) => b.hash(state),
        }
    }
}

impl PartialEq for sqlite::Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (sqlite::Value::Null, sqlite::Value::Null) => true,
            (sqlite::Value::Integer(a), sqlite::Value::Integer(b)) => a == b,
            (sqlite::Value::Real(a), sqlite::Value::Real(b)) => a == b,
            (sqlite::Value::Text(a), sqlite::Value::Text(b)) => a == b,
            (sqlite::Value::Blob(a), sqlite::Value::Blob(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for sqlite::Value {}

impl mysql::Guest for Component {
    type Connection = MySqlConnection;
}

struct MySqlConnection;

impl mysql::GuestConnection for MySqlConnection {
    fn open(address: String) -> Result<mysql::Connection, mysql::Error> {
        let _ = address;
        Err(mysql::Error::Other("not yet implemented".into()))
    }

    fn query(
        &self,
        statement: String,
        params: Vec<mysql::ParameterValue>,
    ) -> Result<mysql::RowSet, mysql::Error> {
        let _ = (statement, params);
        Err(mysql::Error::Other("not yet implemented".into()))
    }

    fn execute(
        &self,
        statement: String,
        params: Vec<mysql::ParameterValue>,
    ) -> Result<(), mysql::Error> {
        let _ = (statement, params);
        Err(mysql::Error::Other("not yet implemented".into()))
    }
}

impl postgres::Guest for Component {
    type Connection = PostgresConnection;
}

struct PostgresConnection;

impl postgres::GuestConnection for PostgresConnection {
    fn open(address: String) -> Result<postgres::Connection, postgres::Error> {
        let _ = address;
        Err(postgres::Error::Other("not yet implemented".into()))
    }

    fn query(
        &self,
        statement: String,
        params: Vec<postgres::ParameterValue>,
    ) -> Result<postgres::RowSet, postgres::Error> {
        let _ = (statement, params);
        Err(postgres::Error::Other("not yet implemented".into()))
    }

    fn execute(
        &self,
        statement: String,
        params: Vec<postgres::ParameterValue>,
    ) -> Result<u64, postgres::Error> {
        let _ = (statement, params);
        Err(postgres::Error::Other("not yet implemented".into()))
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
        let url_allowed = manifest::AppManifest::allows_url(&address, "mqtt")
            .map_err(|_| mqtt::Error::InvalidAddress)?;
        if !url_allowed {
            return Err(mqtt::Error::ConnectionFailed(format!(
                "address {address} is not permitted"
            )));
        }
        let _ = (username, password, keep_alive_interval_in_secs);
        Ok(mqtt::Connection::new(MqttConnection))
    }

    fn publish(
        &self,
        topic: String,
        payload: mqtt::Payload,
        qos: mqtt::Qos,
    ) -> Result<(), mqtt::Error> {
        let _ = (topic, payload, qos);
        Ok(())
    }
}

impl variables::Guest for Component {
    fn get(name: String) -> Result<String, variables::Error> {
        let key = spin_expressions::Key::new(&name)
            .map_err(|_| variables::Error::InvalidName(name.clone()))?;
        let component_id = manifest::AppManifest::get_component_id().expect("no component id set");
        VARIABLE_RESOLVER.with(|resolver| {
            let name = key.as_str().to_owned();
            let fut = resolver
                .as_ref()
                .map_err(|e| variables::Error::Other(e.to_string()))?
                .resolve(component_id.as_ref(), key);
            futures::executor::block_on(fut).map_err(|_| variables::Error::Undefined(name))
        })
    }
}

thread_local! {
    /// The global variable resolver.
    static VARIABLE_RESOLVER: LazyCell<Result<spin_expressions::ProviderResolver, spin_expressions::Error>> = LazyCell::new(|| {
        let variables = manifest::AppManifest::get()
            .variables
            .into_iter()
            .map(|(k, v)| {
                let v = spin_locked_app::Variable {
                    default: v.default,
                    secret: v.secret,
                };
                (k.to_string(), v)
            });
        let mut resolver = spin_expressions::ProviderResolver::new(variables)?;
        let component = manifest::AppManifest::get_component().expect("no component set");
        let component_id = manifest::AppManifest::get_component_id().expect("no component id set");
        resolver
            .add_component_variables(
                component_id,
                component
                    .variables
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v)),
            )?;
        resolver.add_provider(Box::new(UserGivenProvider));
        Ok(resolver)

    });

    /// The user-defined variables.
    ///
    /// These are the application variables the user is supposed to provide.
    static USER_DEFINED_VARIABLES: LazyCell<RefCell<HashMap<String, String>>> = LazyCell::new(|| {
        RefCell::new(HashMap::new())
    });
}

/// A variable provider populated through the `fermyon:spin-test-virt/variable` interface.
#[derive(Debug)]
struct UserGivenProvider;

#[async_trait::async_trait]
impl spin_expressions::Provider for UserGivenProvider {
    async fn get(&self, key: &spin_expressions::Key) -> anyhow::Result<Option<String>> {
        Ok(USER_DEFINED_VARIABLES.with(|vars| vars.borrow().get(key.as_str()).cloned()))
    }
}

impl spin_test_virt::variables::Guest for Component {
    fn set(key: String, value: String) {
        USER_DEFINED_VARIABLES.with(|vars| {
            vars.borrow_mut().insert(key, value);
        });
    }
}

impl virt_key_value::Guest for Component {
    fn calls() -> Vec<(String, Vec<virt_key_value::Call>)> {
        Stores::get()
            .read()
            .unwrap()
            .iter()
            .map(|(label, store)| (label.clone(), store.read_calls()))
            .collect()
    }

    fn reset_calls() {
        for store in Stores::get().read().unwrap().values() {
            store.clear_calls();
        }
    }

    type Store = VirtKeyValueStore;
}

struct VirtKeyValueStore {
    inner: KeyValueStore,
}

impl VirtKeyValueStore {
    fn new(inner: KeyValueStore) -> Self {
        Self { inner }
    }
}

impl virt_key_value::GuestStore for VirtKeyValueStore {
    fn open(label: String) -> spin_test_virt::key_value::Store {
        let mut stores = Stores::get().write().unwrap();
        let key_value = stores
            .entry(label.clone())
            .or_insert_with(|| KeyValueStore::new(label));
        virt_key_value::Store::new(VirtKeyValueStore::new(key_value.clone()))
    }

    fn label(&self) -> String {
        self.inner.label.clone()
    }

    fn get(&self, key: String) -> Option<Vec<u8>> {
        self.inner.get(&key)
    }

    fn set(&self, key: String, value: Vec<u8>) {
        self.inner.set(key, value);
    }

    fn delete(&self, key: String) {
        self.inner.delete(&key);
    }
}

bindings::export!(Component with_types_in bindings);
