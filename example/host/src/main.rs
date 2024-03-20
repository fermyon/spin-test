use wasmtime_wasi::preview2::{self, WasiView};
use wasmtime_wasi_http::{proxy, WasiHttpView};

mod bindings {
    wasmtime::component::bindgen!({
        world: "virtualized-component",
        path:  "../../wit",
        async: true
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

    // Set state of the key-value store
    let virtualized = VirtualizedComponent::open(&mut runtime, &instance, "example").await;
    virtualized.set(&mut runtime, "hello", b"world").await;

    // Make an HTTP request
    let req = hyper::Request::builder()
        .uri("http://example.com:8080/test-path")
        .method(http::Method::GET)
        .body(body::empty())
        .unwrap();
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

    // Print the response body
    match receiver.await.unwrap() {
        Ok(resp) => {
            use http_body_util::BodyExt;
            let (_, body) = resp.into_parts();
            let collected = BodyExt::collect(body).await.unwrap();
            println!(
                "The Spin App's Body: {}",
                String::from_utf8_lossy(&collected.to_bytes())
            );
        }
        Err(_) => todo!(),
    }
}

struct Runtime {
    engine: wasmtime::Engine,
    store: wasmtime::Store<Data>,
    linker: wasmtime::component::Linker<Data>,
}

impl Runtime {
    fn create() -> Self {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        let engine = wasmtime::Engine::new(&config).unwrap();
        let store = wasmtime::Store::new(&engine, Data::new());
        let mut linker = wasmtime::component::Linker::new(&engine);
        proxy::add_to_linker(&mut linker).unwrap();
        preview2::bindings::cli::environment::add_to_linker(&mut linker, |x| x).unwrap();
        preview2::bindings::cli::exit::add_to_linker(&mut linker, |x| x).unwrap();
        preview2::bindings::filesystem::types::add_to_linker(&mut linker, |x| x).unwrap();
        preview2::bindings::filesystem::preopens::add_to_linker(&mut linker, |x| x).unwrap();
        Self {
            engine,
            store,
            linker,
        }
    }
}

struct VirtualizedComponent {
    component: bindings::VirtualizedComponent,
    resource: wasmtime::component::ResourceAny,
}

impl VirtualizedComponent {
    async fn open(
        runtime: &mut Runtime,
        instance: &wasmtime::component::Instance,
        store_name: &str,
    ) -> Self {
        let virtualized =
            bindings::VirtualizedComponent::new(&mut runtime.store, &instance).unwrap();
        let key_value = virtualized
            .fermyon_spin_key_value()
            .store()
            .call_open(&mut runtime.store, store_name)
            .await
            .unwrap()
            .unwrap();
        Self {
            component: virtualized,
            resource: key_value,
        }
    }

    async fn set(&self, runtime: &mut Runtime, key: &str, value: &[u8]) {
        self.component
            .fermyon_spin_key_value()
            .store()
            .call_set(&mut runtime.store, self.resource, key, value)
            .await
            .unwrap()
            .unwrap();
    }
}

struct Data {
    table: wasmtime::component::ResourceTable,
    ctx: preview2::WasiCtx,
    http_ctx: wasmtime_wasi_http::WasiHttpCtx,
}

impl Data {
    fn new() -> Self {
        Self {
            table: wasmtime::component::ResourceTable::default(),
            ctx: preview2::WasiCtxBuilder::new().build(),
            http_ctx: wasmtime_wasi_http::WasiHttpCtx,
        }
    }
}

impl WasiView for Data {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut preview2::WasiCtx {
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
