use anyhow::Context as _;

mod bindings {
    wasmtime::component::bindgen!({
        world: "runner",
        path: "host-wit",
        with: {
            "wasi:io/poll": wasmtime_wasi::bindings::io::poll,
            "wasi:io/error": wasmtime_wasi::bindings::io::error,
            "wasi:io/streams": wasmtime_wasi::bindings::io::streams,
            "wasi:clocks/monotonic-clock": wasmtime_wasi::bindings::clocks::monotonic_clock,
            "wasi:http/types": wasmtime_wasi_http::bindings::http::types,
            "fermyon:spin-test/http-helper/response-receiver": super::ResponseReceiver,
        }
    });
}

/// The `spin-test` runtime
pub struct Runtime {
    store: wasmtime::Store<Data>,
    linker: wasmtime::component::Linker<Data>,
    component: wasmtime::component::Component,
}

impl Runtime {
    /// Create a new runtime
    pub fn instantiate(manifest: String, composed_component: &[u8]) -> anyhow::Result<Self> {
        if std::env::var("SPIN_TEST_DUMP_COMPOSITION").is_ok() {
            let _ = std::fs::write("composition.wasm", composed_component);
        }
        let engine = wasmtime::Engine::default();
        let store = wasmtime::Store::new(&engine, Data::new(manifest));

        let component = wasmtime::component::Component::new(&engine, composed_component)
            .context("composed component was an invalid Wasm component")?;

        let mut linker = wasmtime::component::Linker::<Data>::new(&engine);
        wasmtime_wasi::command::sync::add_to_linker(&mut linker)
            .context("failed to link to wasi")?;
        bindings::Runner::add_to_linker(&mut linker, |x| x)
            .context("failed to link to test runner world")?;

        Ok(Self {
            component,
            store,
            linker,
        })
    }

    /// Run the test component
    pub fn run(&mut self, test_name: Option<&str>) -> anyhow::Result<()> {
        match test_name {
            Some(test_name) => {
                let test_instance = self
                    .linker
                    .instantiate(&mut self.store, &self.component)
                    .context("failed to instantiate spin-test composition")?;

                let test_func = test_instance
                    .get_typed_func::<(), ()>(&mut self.store, test_name)
                    .with_context(|| format!("failed to get test function '{test_name}'"))?;

                test_func
                    .call(&mut self.store, ())
                    .context(format!("test '{test_name}' failed "))
            }
            None => {
                let (runner, _) = bindings::Runner::instantiate(
                    &mut self.store,
                    &self.component,
                    &self.linker,
                )
                .context("failed to instantiate spin-test composition as test runner world")?;

                runner.call_run(&mut self.store)
            }
        }
    }
}

/// Store specific data
struct Data {
    table: wasmtime_wasi::ResourceTable,
    ctx: wasmtime_wasi::WasiCtx,
    manifest: String,
}

impl Data {
    fn new(manifest: String) -> Self {
        let table = wasmtime_wasi::ResourceTable::new();
        let ctx = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .inherit_env()
            .build();
        Self {
            table,
            ctx,
            manifest,
        }
    }
}

impl bindings::RunnerImports for Data {
    fn get_manifest(&mut self) -> wasmtime::Result<String> {
        Ok(self.manifest.clone())
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
