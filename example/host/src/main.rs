use std::sync::Arc;

use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use wasmtime::component::{Instance, Resource};
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

#[tokio::main]
async fn main() {
    // Create a runtime
    let mut runtime = Runtime::create();

    // Load and instantiate the component
    let component_path = std::env::args()
        .skip(1)
        .next()
        .expect("first arg must be a path");
    let component =
        wasmtime::component::Component::from_file(&runtime.engine, component_path).unwrap();
    let (proxy, instance) =
        proxy::Proxy::instantiate_async(&mut runtime.store, &component, &runtime.linker)
            .await
            .unwrap();

    let config = Config::new(&mut runtime, &instance).unwrap();

    // Set state of the key-value store
    let key_value_config = config.key_value_store(&mut runtime, "example").await;
    key_value_config.set(&mut runtime, "hello", b"world").await;

    // Set a response for an HTTP request
    let handler = config.outbound_http_handler();
    let response = http::Response::builder()
        .status(200)
        .body(body::empty())
        .unwrap();
    handler
        .set_response(&mut runtime, "https://example.com", response)
        .await
        .unwrap();

    // Make an HTTP request
    let req = hyper::Request::builder()
        .uri("http://example.com:8080/test-path")
        .method(http::Method::GET)
        .body(body::empty())
        .unwrap();
    let response = perform_request(&mut runtime, &proxy, req).await;

    // Print the response body
    match response {
        Ok(resp) => {
            use http_body_util::BodyExt;
            let (_, body) = resp.into_parts();
            let body = body.collect().await.unwrap().to_bytes();
            println!("The Spin App's Body: {}", String::from_utf8_lossy(&body));
        }
        Err(e) => {
            eprintln!("Spin app failed: {e:?}");
        }
    }
}

/// Call the Spin application with an HTTP request.
async fn perform_request(
    runtime: &mut Runtime,
    proxy: &proxy::Proxy,
    req: hyper::Request<BoxBody<Bytes, ErrorCode>>,
) -> Result<http::Response<BoxBody<Bytes, ErrorCode>>, ErrorCode> {
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

/// The runtime for the Spin application.
struct Runtime {
    engine: wasmtime::Engine,
    store: wasmtime::Store<Data>,
    linker: wasmtime::component::Linker<Data>,
}

impl Runtime {
    /// Create a new runtime.
    fn create() -> Self {
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
struct Config {
    inner: Arc<bindings::Config>,
}

impl Config {
    /// Create a new `Config`.
    fn new(runtime: &mut Runtime, instance: &Instance) -> Result<Self, wasmtime::Error> {
        let inner = Arc::new(bindings::Config::new(&mut runtime.store, instance)?);
        Ok(Self { inner })
    }

    /// Open a key-value store.
    async fn key_value_store(&self, runtime: &mut Runtime, store_name: &str) -> KeyValueConfig {
        KeyValueConfig::open(self.clone(), runtime, store_name).await
    }

    fn outbound_http_handler(&self) -> OutboundHttpHandler {
        OutboundHttpHandler::new(self.clone())
    }
}

/// A handler for key-value store operations.
struct KeyValueConfig {
    config: Config,
    key_value: wasmtime::component::ResourceAny,
}

impl KeyValueConfig {
    /// Open a key-value store.
    async fn open(config: Config, runtime: &mut Runtime, store_name: &str) -> Self {
        let key_value = config
            .inner
            .fermyon_spin_key_value()
            .store()
            .call_open(&mut runtime.store, store_name)
            .await
            .unwrap()
            .unwrap();
        Self { config, key_value }
    }

    /// Set a key/value pair in the store.
    async fn set(&self, runtime: &mut Runtime, key: &str, value: &[u8]) {
        self.config
            .inner
            .fermyon_spin_key_value()
            .store()
            .call_set(&mut runtime.store, self.key_value, key, value)
            .await
            .unwrap()
            .unwrap();
    }
}

/// A handler for outbound HTTP requests.
struct OutboundHttpHandler {
    config: Config,
}

impl OutboundHttpHandler {
    /// Create a new `OutboundHttpHandler`.
    fn new(config: Config) -> Self {
        Self { config }
    }

    /// Set a response for a given URL.
    async fn set_response(
        &self,
        runtime: &mut Runtime,
        url: &str,
        response: http::Response<BoxBody<Bytes, ErrorCode>>,
    ) -> wasmtime::Result<()> {
        let response = response_resource(response, runtime.store.data_mut());
        self.config
            .inner
            .fermyon_spin_virt_http_handler()
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

mod body {
    use http_body_util::{combinators::BoxBody, BodyExt, Empty};
    use wasmtime_wasi_http::body::HyperIncomingBody;

    pub fn empty() -> HyperIncomingBody {
        BoxBody::new(Empty::new().map_err(|_| unreachable!()))
    }
}
