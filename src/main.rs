use std::{cell::RefCell, rc::Rc, sync::Arc};
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

const SPIN_TEST_VIRT: &[u8] = include_bytes!("../example/deps/fermyon/spin-test-virt.wasm");
const WASI_VIRT: &[u8] = include_bytes!("../example/deps/wasi/virt.wasm");
const ROUTER: &[u8] = include_bytes!("../example/deps/fermyon/router.wasm");

fn main() {
    env_logger::init();
    let test_path = std::env::args()
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

    let test = std::fs::read(test_path).unwrap();
    let app = std::fs::read(app_path).unwrap();
    let app = spin_componentize::componentize_if_necessary(&app)
        .unwrap()
        .into_owned();

    let encoded = encode_composition(app, test);

    let mut runtime = Runtime::new(raw_manifest, &encoded);
    runtime.call_run().unwrap();
}

fn encode_composition(app: Vec<u8>, test: Vec<u8>) -> Vec<u8> {
    let composition = Composition::new();
    let virt = composition
        .instantiate("virt", SPIN_TEST_VIRT, Vec::new())
        .unwrap();
    let wasi_virt = composition
        .instantiate("wasi_virt", WASI_VIRT, Vec::new())
        .unwrap();

    let app_args = vec![
        (
            "fermyon:spin/key-value@2.0.0",
            virt.export("fermyon:spin/key-value@2.0.0").unwrap(),
        ),
        (
            "wasi:http/outgoing-handler@0.2.0",
            virt.export("wasi:http/outgoing-handler@0.2.0").unwrap(),
        ),
        (
            "wasi:cli/environment@0.2.0",
            wasi_virt.export("wasi:cli/environment@0.2.0").unwrap(),
        ),
    ];
    let app = composition.instantiate("app", &app, app_args).unwrap();

    let router_args = vec![
        ("set-component-id", virt.export("set-component-id").unwrap()),
        (
            "wasi:http/incoming-handler@0.2.0",
            app.export("wasi:http/incoming-handler@0.2.0").unwrap(),
        ),
    ];
    let router = composition
        .instantiate("router", ROUTER, router_args)
        .unwrap();

    let test_args = vec![
        (
            "wasi:http/incoming-handler@0.2.0",
            router.export("wasi:http/incoming-handler@0.2.0").unwrap(),
        ),
        (
            "fermyon:spin/key-value@2.0.0",
            virt.export("fermyon:spin/key-value@2.0.0").unwrap(),
        ),
        (
            "fermyon:spin-test-virt/key-value-calls",
            virt.export("fermyon:spin-test-virt/key-value-calls")
                .unwrap(),
        ),
    ];
    let test = composition.instantiate("test", &test, test_args).unwrap();
    let export = test.export("run").unwrap();

    composition.export(export, "run").unwrap();
    composition.encode().unwrap()
}

struct Composition {
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
}

impl Composition {
    fn new() -> Self {
        Self {
            graph: Rc::new(RefCell::new(wac_graph::CompositionGraph::new())),
        }
    }

    pub fn instantiate(
        &self,
        name: &str,
        bytes: &[u8],
        arguments: Vec<(&str, Export)>,
    ) -> anyhow::Result<Instance> {
        let package =
            wac_graph::types::Package::from_bytes(name, None, Arc::new(bytes.to_owned()))?;
        let package = self.graph.borrow_mut().register_package(package)?;
        let instance = self.graph.borrow_mut().instantiate(package)?;
        for (arg_name, arg) in arguments {
            self.graph
                .borrow_mut()
                .set_instantiation_argument(instance, arg_name, arg.node)?;
        }
        Ok(Instance {
            graph: self.graph.clone(),
            node: instance,
        })
    }

    fn export(&self, export: Export, name: &str) -> anyhow::Result<()> {
        Ok(self.graph.borrow_mut().export(export.node, name)?)
    }

    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self
            .graph
            .borrow_mut()
            .encode(wac_graph::EncodeOptions::default())?)
    }
}

struct Instance {
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
    node: wac_graph::NodeId,
}

impl Instance {
    fn export(&self, name: &str) -> anyhow::Result<Export> {
        let node = self
            .graph
            .borrow_mut()
            .alias_instance_export(self.node, name)?;

        Ok(Export { node })
    }
}

struct Export {
    node: wac_graph::NodeId,
}

struct Runtime {
    store: wasmtime::Store<Data>,
    runner: bindings::Runner,
}

impl Runtime {
    fn new(manifest: String, composed_component: &[u8]) -> Self {
        let engine = wasmtime::Engine::default();
        let mut store = wasmtime::Store::new(&engine, Data::new(manifest));

        let component = wasmtime::component::Component::new(&engine, composed_component).unwrap();

        let mut linker = wasmtime::component::Linker::new(&engine);
        wasmtime_wasi::command::sync::add_to_linker(&mut linker).unwrap();
        wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |x| x).unwrap();
        bindings::Runner::add_to_linker(&mut linker, |x| x).unwrap();

        let (runner, _) =
            bindings::Runner::instantiate(&mut store, &component, &mut linker).unwrap();
        Self { store, runner }
    }

    fn call_run(&mut self) -> anyhow::Result<()> {
        self.runner.call_run(&mut self.store)
    }
}

/// Store specific data
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
