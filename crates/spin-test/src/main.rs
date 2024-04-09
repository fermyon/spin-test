use std::sync::Arc;
use wasmtime_wasi_http::{
    bindings::http::incoming_handler::{IncomingRequest, ResponseOutparam},
    WasiHttpView,
};

mod bindings {
    wasmtime::component::bindgen!({
            world: "runner",
            path: "host-wit",
            with: {
                "wasi:io/poll": wasmtime_wasi::bindings::io::poll,
                "wasi:io/error": wasmtime_wasi::bindings::io::error,
                "wasi:io/streams": wasmtime_wasi::bindings::io::streams,
                "wasi:clocks/monotonic-clock": wasmtime_wasi::bindings::clocks::monotonic_clock,
                "wasi:http/types": wasmtime_wasi_http::bindings::http::types
            }
    });
}

const SPIN_TEST_VIRT: &[u8] = include_bytes!("../../../example/deps/fermyon/spin-test-virt.wasm");
const WASI_VIRT: &[u8] = include_bytes!("../../../example/deps/wasi/virt.wasm");
const ROUTER: &[u8] = include_bytes!("../../../example/deps/fermyon/router.wasm");

fn main() {
    env_logger::init();
    let test = std::env::args()
        .nth(1)
        .expect("second argument should be the path to the test wasm");
    let manifest_path =
        spin_common::paths::resolve_manifest_file_path(spin_common::paths::DEFAULT_MANIFEST_FILE)
            .unwrap();
    let raw_manifest = std::fs::read_to_string(&manifest_path).unwrap();
    let manifest = spin_manifest::manifest_from_str(&raw_manifest).unwrap();
    let app_path = match &manifest.components.first().as_ref().unwrap().1.source {
        spin_manifest::schema::v2::ComponentSource::Local(path) => path,
        spin_manifest::schema::v2::ComponentSource::Remote { .. } => {
            todo!("handle remote component sources")
        }
    };

    let app = std::fs::read(app_path).unwrap();
    let app = spin_componentize::componentize_if_necessary(&app)
        .unwrap()
        .into_owned();
    let app = wac_graph::types::Package::from_bytes("app", None, app).unwrap();
    let test = wac_graph::types::Package::from_file("test", None, test).unwrap();
    let virt = wac_graph::types::Package::from_bytes(
        "spin-test-virt",
        None,
        Arc::new(SPIN_TEST_VIRT.to_owned()),
    )
    .unwrap();
    let wasi_virt =
        wac_graph::types::Package::from_bytes("wasi-virt", None, Arc::new(WASI_VIRT.to_owned()))
            .unwrap();
    let router =
        wac_graph::types::Package::from_bytes("router", None, Arc::new(ROUTER.to_owned())).unwrap();

    let mut graph = wac_graph::CompositionGraph::new();
    let app = graph.register_package(app).unwrap();
    let test = graph.register_package(test).unwrap();
    let virt = graph.register_package(virt).unwrap();
    let wasi_virt = graph.register_package(wasi_virt).unwrap();
    let router = graph.register_package(router).unwrap();

    let virt = graph.instantiate(virt).unwrap();
    let app = graph.instantiate(app).unwrap();
    let wasi_virt = graph.instantiate(wasi_virt).unwrap();
    let router = graph.instantiate(router).unwrap();
    let test = graph.instantiate(test).unwrap();

    let key_value = graph
        .alias_instance_export(virt, "fermyon:spin/key-value@2.0.0")
        .unwrap();
    graph
        .set_instantiation_argument(app, "fermyon:spin/key-value@2.0.0", key_value)
        .unwrap();

    let outgoing_handler = graph
        .alias_instance_export(virt, "wasi:http/outgoing-handler@0.2.0")
        .unwrap();
    graph
        .set_instantiation_argument(app, "wasi:http/outgoing-handler@0.2.0", outgoing_handler)
        .unwrap();

    let env = graph
        .alias_instance_export(wasi_virt, "wasi:cli/environment@0.2.0")
        .unwrap();
    graph
        .set_instantiation_argument(app, "wasi:cli/environment@0.2.0", env)
        .unwrap();

    let incoming_handler = graph
        .alias_instance_export(app, "wasi:http/incoming-handler@0.2.0")
        .unwrap();
    graph
        .set_instantiation_argument(router, "wasi:http/incoming-handler@0.2.0", incoming_handler)
        .unwrap();

    let set_component_id = graph
        .alias_instance_export(virt, "set-component-id")
        .unwrap();
    graph
        .set_instantiation_argument(router, "set-component-id", set_component_id)
        .unwrap();

    let incoming_handler_export = graph
        .alias_instance_export(router, "wasi:http/incoming-handler@0.2.0")
        .unwrap();
    let key_value_export = graph
        .alias_instance_export(virt, "fermyon:spin/key-value@2.0.0")
        .unwrap();
    let key_value_calls_export = graph
        .alias_instance_export(virt, "fermyon:spin-test-virt/key-value-calls")
        .unwrap();

    graph
        .set_instantiation_argument(test, "fermyon:spin/key-value@2.0.0", key_value_export)
        .unwrap();
    graph
        .set_instantiation_argument(
            test,
            "wasi:http/incoming-handler@0.2.0",
            incoming_handler_export,
        )
        .unwrap();
    graph
        .set_instantiation_argument(
            test,
            "fermyon:spin-test-virt/key-value-calls",
            key_value_calls_export,
        )
        .unwrap();

    let run_export = graph.alias_instance_export(test, "run").unwrap();

    graph.export(run_export, "run").unwrap();

    let encoded = graph.encode(wac_graph::EncodeOptions::default()).unwrap();
    let engine = wasmtime::Engine::default();
    let mut store = wasmtime::Store::new(&engine, Data::new(raw_manifest));

    let component = wasmtime::component::Component::new(&engine, encoded).unwrap();

    let mut linker = wasmtime::component::Linker::new(&engine);
    wasmtime_wasi::command::sync::add_to_linker(&mut linker).unwrap();
    wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |x| x).unwrap();
    bindings::Runner::add_to_linker(&mut linker, |x| x).unwrap();

    let (bar, _) = bindings::Runner::instantiate(&mut store, &component, &mut linker).unwrap();
    bar.call_run(&mut store).unwrap();
}

impl bindings::RunnerImports for Data {
    fn get_manifest(&mut self) -> wasmtime::Result<String> {
        Ok(self.manifest.clone())
    }
}

impl bindings::fermyon::spin_test::http_helper::Host for Data {
    fn new_request(&mut self) -> wasmtime::Result<wasmtime::component::Resource<IncomingRequest>> {
        let req = hyper::Request::builder()
            .method("GET")
            .uri("http://example.com?user_id=123")
            .body(body::empty())
            .unwrap();
        self.new_incoming_request(req)
    }

    fn new_response(
        &mut self,
    ) -> wasmtime::Result<wasmtime::component::Resource<ResponseOutparam>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        // TODO
        Box::leak(Box::new(rx));
        self.new_response_outparam(tx)
    }
}

struct Data {
    table: wasmtime_wasi::ResourceTable,
    ctx: wasmtime_wasi::WasiCtx,
    http_ctx: wasmtime_wasi_http::WasiHttpCtx,
    manifest: String,
}

impl Data {
    fn new(manifest: String) -> Self {
        let table = wasmtime_wasi::ResourceTable::new();
        let ctx = wasmtime_wasi::WasiCtxBuilder::new().inherit_stdio().build();
        Self {
            table,
            ctx,
            http_ctx: wasmtime_wasi_http::WasiHttpCtx,
            manifest,
        }
    }
}

impl wasmtime_wasi_http::WasiHttpView for Data {
    fn ctx(&mut self) -> &mut wasmtime_wasi_http::WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }
}

impl wasmtime_wasi::WasiView for Data {
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.ctx
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
