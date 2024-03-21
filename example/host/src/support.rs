use std::{path::Path, sync::Arc};

use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use wasmtime::component::{self, Instance, Resource};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{
    bindings::http::outgoing_handler::{ErrorCode, FutureIncomingResponse},
    proxy,
    types::IncomingResponseInternal,
    WasiHttpView,
};

mod bindings {
    wasmtime::component::bindgen!({
        world: "config",
        path:  "../../wit",
        async: true,
        with: {
            "wasi:http/types": wasmtime_wasi_http::bindings::wasi::http::types,
        },
    });
}

/// Call the Spin application with an HTTP request.
pub async fn perform_request(
    runtime: &mut Runtime,
    instance: &Instance,
    req: hyper::Request<BoxBody<Bytes, ErrorCode>>,
) -> Result<http::Response<BoxBody<Bytes, ErrorCode>>, ErrorCode> {
    let proxy = wasmtime_wasi_http::proxy::Proxy::new(&mut runtime.store, instance).unwrap();
    let req = runtime.store.data_mut().new_incoming_request(req).unwrap();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let out = runtime
        .store
        .data_mut()
        .new_response_outparam(sender)
        .unwrap();
    proxy
        .wasi_http_incoming_handler()
        .call_handle(&mut runtime.store, req, out)
        .await
        .unwrap();

    receiver.await.unwrap()
}

/// Load a Spin application from a file.
pub(crate) async fn load(
    runtime: &mut Runtime,
    component_path: impl AsRef<Path>,
) -> wasmtime::Result<Instance> {
    let component = component::Component::from_file(&runtime.engine, component_path)?;
    let (_, instance) =
        bindings::Config::instantiate_async(&mut runtime.store, &component, &runtime.linker)
            .await?;
    Ok(instance)
}

/// The runtime for the Spin application.
pub struct Runtime {
    engine: wasmtime::Engine,
    store: wasmtime::Store<Data>,
    linker: wasmtime::component::Linker<Data>,
}

impl Runtime {
    /// Create a new runtime.
    pub fn create() -> Self {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        let engine = wasmtime::Engine::new(&config).unwrap();
        let store = wasmtime::Store::new(&engine, Data::new());
        let mut linker = wasmtime::component::Linker::new(&engine);
        proxy::add_to_linker(&mut linker).unwrap();
        wasmtime_wasi::bindings::cli::environment::add_to_linker(&mut linker, |x| x).unwrap();
        wasmtime_wasi::bindings::cli::exit::add_to_linker(&mut linker, |x| x).unwrap();
        wasmtime_wasi::bindings::filesystem::types::add_to_linker(&mut linker, |x| x).unwrap();
        wasmtime_wasi::bindings::filesystem::preopens::add_to_linker(&mut linker, |x| x).unwrap();
        Self {
            engine,
            store,
            linker,
        }
    }
}

/// Configuration for how the Spin APIs will behave when called.
#[derive(Clone)]
pub struct Config {
    inner: Arc<bindings::Config>,
}

impl Config {
    /// Create a new `Config`.
    pub fn new(runtime: &mut Runtime, instance: &Instance) -> wasmtime::Result<Self> {
        let inner = Arc::new(bindings::Config::new(&mut runtime.store, instance)?);
        Ok(Self { inner })
    }

    /// Open a key-value store.
    pub async fn key_value_store(
        &self,
        runtime: &mut Runtime,
        store_name: &str,
    ) -> wasmtime::Result<KeyValueConfig> {
        KeyValueConfig::open(self.clone(), runtime, store_name).await
    }

    pub fn outbound_http_handler(&self) -> OutboundHttpHandler {
        OutboundHttpHandler::new(self.clone())
    }
}

/// A handler for key-value store operations.
pub struct KeyValueConfig {
    config: Config,
    key_value: wasmtime::component::ResourceAny,
}

impl KeyValueConfig {
    /// Open a key-value store.
    pub async fn open(
        config: Config,
        runtime: &mut Runtime,
        store_name: &str,
    ) -> wasmtime::Result<Self> {
        let key_value = config
            .inner
            .fermyon_spin_key_value()
            .store()
            .call_open(&mut runtime.store, store_name)
            .await??;
        Ok(Self { config, key_value })
    }

    /// Set a key/value pair in the store.
    pub async fn set(
        &self,
        runtime: &mut Runtime,
        key: &str,
        value: &[u8],
    ) -> wasmtime::Result<()> {
        self.config
            .inner
            .fermyon_spin_key_value()
            .store()
            .call_set(&mut runtime.store, self.key_value, key, value)
            .await??;
        Ok(())
    }
}

/// A handler for outbound HTTP requests.
pub struct OutboundHttpHandler {
    config: Config,
}

impl OutboundHttpHandler {
    /// Create a new `OutboundHttpHandler`.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Set a response for a given URL.
    pub async fn set_response(
        &self,
        runtime: &mut Runtime,
        url: &str,
        response: http::Response<BoxBody<Bytes, ErrorCode>>,
    ) -> wasmtime::Result<()> {
        let response = response_resource(response, runtime.store.data_mut());
        self.config
            .inner
            .fermyon_spin_test_virt_http_handler()
            .call_set_response(&mut runtime.store, url, response)
            .await
    }
}

/// Create a `FutureIncomingResponse` resource from an `http::Response`.
fn response_resource(
    response: http::Response<BoxBody<Bytes, ErrorCode>>,
    view: &mut impl WasiView,
) -> Resource<FutureIncomingResponse> {
    let task = tokio::spawn(async move {
        let worker = tokio::spawn(async { () });
        let response = IncomingResponseInternal {
            resp: response,
            worker: Arc::new(worker.into()),
            between_bytes_timeout: std::time::Duration::from_secs(0),
        };

        Ok(Ok(response))
    });
    let handle: wasmtime_wasi::AbortOnDropJoinHandle<_> = task.into();
    let response = wasmtime_wasi_http::types::HostFutureIncomingResponse::new(handle);
    let response = WasiView::table(view).push(response).unwrap();
    Resource::new_own(response.rep())
}

struct Data {
    table: wasmtime::component::ResourceTable,
    ctx: WasiCtx,
    http_ctx: wasmtime_wasi_http::WasiHttpCtx,
}

impl Data {
    fn new() -> Self {
        Self {
            table: wasmtime::component::ResourceTable::default(),
            ctx: WasiCtxBuilder::new().build(),
            http_ctx: wasmtime_wasi_http::WasiHttpCtx,
        }
    }
}

impl WasiView for Data {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl WasiHttpView for Data {
    fn ctx(&mut self) -> &mut wasmtime_wasi_http::WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.table
    }
}

pub mod body {
    use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
    use wasmtime_wasi_http::body::HyperIncomingBody;

    pub fn empty() -> HyperIncomingBody {
        BoxBody::new(Empty::new().map_err(|_| unreachable!()))
    }

    pub fn full(body: Vec<u8>) -> HyperIncomingBody {
        BoxBody::new(Full::new(body.into()).map_err(|_| unreachable!()))
    }
}
