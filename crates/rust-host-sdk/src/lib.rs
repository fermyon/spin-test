use std::{path::Path, sync::Arc};

use anyhow::Context;
use bindings::{exports::fermyon::spin_test_virt::key_value_calls, Config};
use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use wasmtime::component::{self, Instance, Resource};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{
    bindings::http::types::{ErrorCode, FutureIncomingResponse},
    proxy,
    types::IncomingResponseInternal,
    WasiHttpView,
};

mod bindings {
    wasmtime::component::bindgen!({
        world: "config",
        path:  "../spin-test-virt/wit",
        async: true,
        with: {
            "wasi:http/types": wasmtime_wasi_http::bindings::wasi::http::types,
        },
    });
}

/// The runtime for the Spin application.
pub struct Spin {
    store: wasmtime::Store<Data>,
    instance: Instance,
}

impl Spin {
    /// Create a new runtime.
    pub async fn create(
        component_path: impl AsRef<Path>,
        manifest_path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        let engine = wasmtime::Engine::new(&config)?;
        let mut store =
            wasmtime::Store::new(&engine, Data::new(std::fs::read_to_string(manifest_path)?));
        let mut linker = wasmtime::component::Linker::new(&engine);
        proxy::add_to_linker(&mut linker)?;
        wasmtime_wasi::bindings::cli::environment::add_to_linker(&mut linker, |x| x)?;
        wasmtime_wasi::bindings::cli::exit::add_to_linker(&mut linker, |x| x)?;
        wasmtime_wasi::bindings::filesystem::types::add_to_linker(&mut linker, |x| x)?;
        wasmtime_wasi::bindings::filesystem::preopens::add_to_linker(&mut linker, |x| x)?;
        bindings::Config::add_root_to_linker(&mut linker, |x| x)?;
        let component = component::Component::from_file(&engine, component_path)?;
        let (_, instance) =
            bindings::Config::instantiate_async(&mut store, &component, &linker).await?;
        Ok(Self { store, instance })
    }

    /// Call the Spin application with an HTTP request.
    pub async fn perform_request(
        &mut self,
        req: hyper::Request<BoxBody<Bytes, ErrorCode>>,
    ) -> anyhow::Result<http::Response<BoxBody<Bytes, ErrorCode>>> {
        // Reset call tracking
        let config = bindings::Config::new(&mut self.store, &self.instance)?;
        config
            .fermyon_spin_test_virt_key_value_calls()
            .call_reset(&mut self.store)
            .await?;

        let proxy = wasmtime_wasi_http::proxy::Proxy::new(&mut self.store, &self.instance)?;
        let req = self.store.data_mut().new_incoming_request(req)?;
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let out = self.store.data_mut().new_response_outparam(sender)?;
        proxy
            .wasi_http_incoming_handler()
            .call_handle(&mut self.store, req, out)
            .await?;

        Ok(receiver
            .await
            .context("issue with response channel")?
            .context("Spin returned an error instead of a response")?)
    }

    /// Open a key-value store.
    pub async fn key_value_store<'a>(
        &'a mut self,
        store_name: &'a str,
    ) -> wasmtime::Result<KeyValueConfig> {
        KeyValueConfig::open(self, store_name).await
    }

    pub fn outbound_http_handler(&mut self) -> OutboundHttpHandler {
        OutboundHttpHandler::new(self)
    }
}

/// A handler for key-value store operations.
pub struct KeyValueConfig<'a> {
    spin: &'a mut Spin,
    key_value: wasmtime::component::ResourceAny,
    store_name: &'a str,
}

impl<'a> KeyValueConfig<'a> {
    /// Open a key-value store.
    pub async fn open(spin: &'a mut Spin, store_name: &'a str) -> wasmtime::Result<Self> {
        let config = bindings::Config::new(&mut spin.store, &spin.instance)?;
        let key_value = config
            .fermyon_spin_key_value()
            .store()
            .call_open(&mut spin.store, store_name)
            .await??;
        Ok(Self {
            spin,
            key_value,
            store_name,
        })
    }

    /// Set a key/value pair in the store.
    pub async fn get(&mut self, key: &str) -> wasmtime::Result<Option<Vec<u8>>> {
        let config = bindings::Config::new(&mut self.spin.store, &self.spin.instance)?;
        let value = config
            .fermyon_spin_key_value()
            .store()
            .call_get(&mut self.spin.store, self.key_value, key)
            .await??;
        Ok(value)
    }

    /// Set a key/value pair in the store.
    pub async fn set(&mut self, key: &str, value: &[u8]) -> wasmtime::Result<()> {
        let config = bindings::Config::new(&mut self.spin.store, &self.spin.instance)?;
        config
            .fermyon_spin_key_value()
            .store()
            .call_set(&mut self.spin.store, self.key_value, key, value)
            .await??;
        Ok(())
    }

    pub fn calls(&mut self) -> anyhow::Result<KeyValueCalls> {
        KeyValueCalls::new(self.spin, self.store_name)
    }
}

/// A handler for outbound HTTP requests.
pub struct OutboundHttpHandler<'a> {
    spin: &'a mut Spin,
}

impl<'a> OutboundHttpHandler<'a> {
    /// Create a new `OutboundHttpHandler`.
    pub fn new(spin: &'a mut Spin) -> Self {
        Self { spin }
    }

    /// Set a response for a given URL.
    pub async fn set_response(
        &mut self,
        url: &str,
        response: http::Response<BoxBody<Bytes, ErrorCode>>,
    ) -> wasmtime::Result<()> {
        let response = response_resource(response, self.spin.store.data_mut())?;
        let config = bindings::Config::new(&mut self.spin.store, &self.spin.instance)?;
        config
            .fermyon_spin_test_virt_http_handler()
            .call_set_response(&mut self.spin.store, url, response)
            .await
    }
}

/// Create a `FutureIncomingResponse` resource from an `http::Response`.
fn response_resource(
    response: http::Response<BoxBody<Bytes, ErrorCode>>,
    view: &mut impl WasiView,
) -> anyhow::Result<Resource<FutureIncomingResponse>> {
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
    Ok(WasiView::table(view).push(response)?)
}

pub use key_value_calls::GetCall;
pub use key_value_calls::SetCall;

impl PartialEq for GetCall {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl PartialEq for SetCall {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.value == other.value
    }
}

/// A tracker of calls
pub struct KeyValueCalls<'a> {
    spin: &'a mut Spin,
    config: Config,
    store: &'a str,
}

impl<'a> KeyValueCalls<'a> {
    /// Create a new `KeyValueCalls`.
    pub fn new(spin: &'a mut Spin, store: &'a str) -> anyhow::Result<Self> {
        let config = bindings::Config::new(&mut spin.store, &spin.instance)?;
        Ok(Self {
            spin,
            config,
            store,
        })
    }

    /// Get the calls made to the key-value store.
    pub async fn get_calls(&mut self) -> anyhow::Result<Vec<key_value_calls::GetCall>> {
        Ok(self
            .config
            .fermyon_spin_test_virt_key_value_calls()
            .call_get(&mut self.spin.store)
            .await?
            .iter()
            .find(|(store, _)| store == self.store)
            .map(|(_, calls)| calls.clone())
            .unwrap_or_default())
    }

    pub async fn set_calls(&mut self) -> anyhow::Result<Vec<key_value_calls::SetCall>> {
        Ok(self
            .config
            .fermyon_spin_test_virt_key_value_calls()
            .call_set(&mut self.spin.store)
            .await?
            .iter()
            .find(|(store, _)| store == self.store)
            .map(|(_, calls)| calls.clone())
            .unwrap_or_default())
    }
}

struct Data {
    table: wasmtime::component::ResourceTable,
    ctx: WasiCtx,
    http_ctx: wasmtime_wasi_http::WasiHttpCtx,
    manifest: String,
}

impl Data {
    fn new(manifest: String) -> Self {
        Self {
            table: wasmtime::component::ResourceTable::default(),
            ctx: WasiCtxBuilder::new().inherit_stdout().build(),
            http_ctx: wasmtime_wasi_http::WasiHttpCtx,
            manifest,
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

#[async_trait::async_trait]
impl bindings::ConfigImports for Data {
    async fn get_manifest(&mut self) -> wasmtime::Result<String> {
        Ok(self.manifest.clone())
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
