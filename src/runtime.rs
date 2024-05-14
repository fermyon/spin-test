use anyhow::Context as _;

use crate::manifest::ManifestInformation;

mod non_dynamic {
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

mod dynamic {
    wasmtime::component::bindgen!({
        world: "dynamic-runner",
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
    manifest: ManifestInformation,
}

impl Runtime {
    /// Create a new runtime
    pub fn instantiate(
        manifest: ManifestInformation,
        composed_component: &[u8],
    ) -> anyhow::Result<Self> {
        if std::env::var("SPIN_TEST_DUMP_COMPOSITION").is_ok() {
            let _ = std::fs::write("composition.wasm", composed_component);
        }
        let engine = wasmtime::Engine::default();
        let store = wasmtime::Store::new(&engine, Data::new(manifest.raw().to_owned()));

        let component = wasmtime::component::Component::new(&engine, composed_component)
            .context("composed component was an invalid Wasm component")?;

        let mut linker = wasmtime::component::Linker::<Data>::new(&engine);
        wasmtime_wasi::command::sync::add_to_linker(&mut linker)
            .context("failed to link to wasi")?;
        non_dynamic::Runner::add_to_linker(&mut linker, |x| x)
            .context("failed to link to test runner world")?;

        Ok(Self {
            store,
            linker,
            component,
            manifest,
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
                let runner = dynamic::DynamicRunner::new(&mut self.store, &test_instance)?;
                self.add_files(runner)?;

                let test_func = test_instance
                    .get_typed_func::<(), ()>(&mut self.store, test_name)
                    .with_context(|| format!("failed to get test function '{test_name}'"))?;

                test_func
                    .call(&mut self.store, ())
                    .context(format!("test '{test_name}' failed "))
            }
            None => {
                let (runner, _) = non_dynamic::Runner::instantiate(
                    &mut self.store,
                    &self.component,
                    &self.linker,
                )
                .context("failed to instantiate spin-test composition as test runner world")?;

                runner.call_run(&mut self.store)
            }
        }
    }

    /// Make all mounted files visible to the WASI virtual filesystem
    fn add_files(&mut self, runner: dynamic::DynamicRunner) -> anyhow::Result<()> {
        /// Make a file visible to the WASI virtual filesystem
        fn add_file<T>(
            store: &mut wasmtime::Store<T>,
            runner: &dynamic::DynamicRunner,
            host_path: &std::path::Path,
            guest_path: &std::path::Path,
        ) -> anyhow::Result<()> {
            let contents = std::fs::read(host_path).context("failed to read file contents")?;
            runner.fermyon_spin_wasi_virt_fs_handler().call_add_file(
                store,
                &guest_path.to_string_lossy(),
                &contents,
            )
        }
        for file in self.manifest.component().files.iter() {
            match file {
                spin_manifest::schema::v2::WasiFilesMount::Pattern(p) => {
                    for path in glob::glob(p).context("failed to glob pattern")? {
                        let path = path.context("failed to read glob entry")?;
                        let path = self.manifest.relative_from(path);
                        if path.is_dir() {
                            for entry in
                                std::fs::read_dir(path).context("failed to read directory")?
                            {
                                let entry = entry.context("failed to read directory entry")?;
                                let path = entry.path();
                                if path.is_file() {
                                    add_file(&mut self.store, &runner, &path, &path)?;
                                }
                            }
                        } else {
                            add_file(&mut self.store, &runner, &path, &path)?;
                        }
                    }
                }
                spin_manifest::schema::v2::WasiFilesMount::Placement {
                    source,
                    destination,
                } => {
                    let source = self.manifest.relative_from(source);
                    if source.is_dir() {
                        let source = source.join("**/*");
                        println!("source: {:?}", source);
                        for path in glob::glob(&source.to_string_lossy())? {
                            let path = path.context("failed to read glob entry")?;
                            if !path.is_file() {
                                continue;
                            }
                            add_file(
                                &mut self.store,
                                &runner,
                                &path,
                                std::path::Path::new(destination),
                            )?;
                        }
                    } else {
                        add_file(
                            &mut self.store,
                            &runner,
                            &source,
                            std::path::Path::new(destination),
                        )?
                    }
                }
            }
        }
        Ok(())
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

impl non_dynamic::RunnerImports for Data {
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
