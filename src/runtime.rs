use anyhow::Context as _;

use crate::manifest::ManifestInformation;

mod non_dynamic {
    wasmtime::component::bindgen!({
        world: "runner",
        path: "host-wit"
    });
}

mod dynamic {
    wasmtime::component::bindgen!({
        world: "dynamic-runner",
        path: "host-wit"
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
        wasmtime_wasi::add_to_linker_sync(&mut linker).context("failed to link to wasi")?;
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
        //TODO(rylev): handle component.exclude_files
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
                    // Expand the glob pattern
                    for host_path in glob::glob(p)
                        .with_context(|| format!("failed to read glob pattern '{p}'"))?
                    {
                        let host_path = host_path.with_context(|| {
                            format!("failed to read glob entry for pattern '{p}'")
                        })?;

                        // Host path is the absolute path to the file
                        let host_path = self.manifest.absolute_from(host_path);
                        // Only add files
                        if !host_path.is_file() {
                            continue;
                        }

                        // Guest path is the path relative to the manifest
                        let guest_path = self.manifest.relative_from(&host_path);
                        add_file(&mut self.store, &runner, &host_path, &guest_path)?;
                    }
                }
                spin_manifest::schema::v2::WasiFilesMount::Placement {
                    // Source can either be a directory or a file
                    source,
                    // Destination is a *directory* relative to the root of the WASI virtual filesystem
                    destination,
                } => {
                    // Destination is always assumed to be an absolute path
                    let destination = format!(
                        "/{}",
                        destination
                            .strip_prefix('/')
                            .unwrap_or(destination.as_str())
                    );
                    let host_path = self.manifest.absolute_from(source);

                    // If the host path is a directory, add all files in the directory
                    if host_path.is_dir() {
                        let host_path = host_path.join("**/*");
                        for host_path in glob::glob(&host_path.to_string_lossy())? {
                            let host_path = host_path.context("failed to read glob entry")?;
                            if !host_path.is_file() {
                                continue;
                            }
                            // Guest path is the path relative to the manifest appended to the destination
                            let guest_path = std::path::Path::new(&destination)
                                // Unwrap should be fine since we know this is a file
                                .join(host_path.file_name().unwrap());

                            add_file(&mut self.store, &runner, &host_path, &guest_path)?;
                        }
                    } else {
                        // Guest path is the path relative to the manifest appended to the destination
                        let guest_path = std::path::Path::new(&destination)
                            // Unwrap should be fine since we know this is a file
                            .join(host_path.file_name().unwrap());
                        add_file(&mut self.store, &runner, &host_path, &guest_path)?
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
