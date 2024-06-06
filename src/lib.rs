mod composition;
mod manifest;
pub mod runtime;

use std::{collections::HashSet, path::PathBuf};

use anyhow::Context;
pub use composition::Composition;
pub use manifest::ManifestInformation;

/// The built `spin-test-virt` component
const SPIN_TEST_VIRT: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/spin_test_virt.wasm"
));
/// The built `router` component
const ROUTER: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/router.wasm"
));

/// A Wasm component
pub struct Component {
    bytes: Vec<u8>,
    path: PathBuf,
}

impl Component {
    /// Read a component from a file.
    ///
    /// If the file is a Wasm module, it will be componentized.
    pub fn from_file(path: PathBuf) -> anyhow::Result<Self> {
        let bytes = std::fs::read(&path)
            .with_context(|| format!("failed to read component binary at '{}'", path.display()))?;
        let bytes = spin_componentize::componentize_if_necessary(&bytes)
            .context("failed to turn module into a component")?
            .into_owned();
        Ok(Self { bytes, path })
    }
}

/// Encode a composition of an app component and a test component
pub fn perform_composition(
    app_component: Component,
    test_component: Component,
    test_target: &TestTarget,
) -> anyhow::Result<Vec<u8>> {
    let composition = Composition::new();

    // Instantiate the `virt` component
    let virt = instantiate_virt(&composition)?;

    // Instantiate the `app` component with various exports from the virt instance
    let app = instantiate_app(&composition, app_component, &virt)?;

    // Instantiate the `router` component
    let router = instantiate_router(&composition, &virt, app)?;

    // Instantiate the `test` component
    let test = instantiate_test(&composition, test_component, router, virt)?;

    match test_target {
        TestTarget::AdHoc { exports } => {
            for test_export in exports {
                let export = test
                    .export(test_export)
                    .context("failed to export '{test_export}' function from test component")?
                    .context(
                        "test component must contain '{test_export}' function but it did not",
                    )?;

                composition
                    .export(export, test_export)
                    .context("failed to export '{test_export}' function from composition")?;
            }
        }
        TestTarget::TestWorld { .. } => {
            let export = test
                .export("run")
                .context("failed to export 'run' function from test component")?
                .context("test component must contain 'run' function but it did not")?;

            composition
                .export(export, "run")
                .context("failed to export 'run' function from composition")?;
        }
    }

    composition
        .encode(true)
        .context("failed to encode composition")
}

/// Virtualize app component with virtualized environment and router
pub fn virtualize_app(app_component: Component) -> anyhow::Result<Vec<u8>> {
    let composition = Composition::new();

    // Instantiate the `virt` component
    let virt = instantiate_virt(&composition)?;
    let export = |name| {
        let instance = virt.export(name).unwrap().unwrap();
        composition.export(instance, name).unwrap();
    };
    export("fermyon:spin-wasi-virt/http-helper");
    export("wasi:http/types@0.2.0");
    export("wasi:clocks/monotonic-clock@0.2.0");
    export("wasi:io/streams@0.2.0");
    export("wasi:io/error@0.2.0");
    export("wasi:io/poll@0.2.0");

    // Instantiate the `app` component with various exports from the virt instance
    let app = instantiate_app(&composition, app_component, &virt)?;

    // Instantiate the `router` component
    let router = instantiate_router(&composition, &virt, app)?;

    let export = router
        .export("wasi:http/incoming-handler@0.2.0")
        .context("failed to export 'wasi:http/incoming-handler@0.2.0' from router")?
        .context("router must export 'wasi:http/incoming-handler@0.2.0' but it did not")?;

    composition
        .export(export, "wasi:http/incoming-handler@0.2.0")
        .unwrap();

    composition
        .encode(true)
        .context("failed to encode composition")
}

fn instantiate_test(
    composition: &Composition,
    test_component: Component,
    router: composition::Instance,
    virt: composition::Instance,
) -> Result<composition::Instance, anyhow::Error> {
    // Get args from `router` and `virt` instances
    let router_args = [Ok((
        "wasi:http/incoming-handler@0.2.0",
        export_item(&router, "wasi:http/incoming-handler@0.2.0")?,
    ))]
    .into_iter();
    let virt_args = [
        "fermyon:spin-wasi-virt/http-handler",
        "fermyon:spin/sqlite@2.0.0",
        "fermyon:spin-test-virt/sqlite",
        "fermyon:spin-test-virt/key-value",
        "fermyon:spin/key-value@2.0.0",
        "wasi:io/error@0.2.0",
        "wasi:io/streams@0.2.0",
        "wasi:io/poll@0.2.0",
        "wasi:clocks/monotonic-clock@0.2.0",
        "wasi:clocks/wall-clock@0.2.0",
        "wasi:filesystem/types@0.2.0",
        "wasi:filesystem/preopens@0.2.0",
        "wasi:cli/stdin@0.2.0",
        "wasi:cli/stderr@0.2.0",
        "wasi:cli/stdout@0.2.0",
        "wasi:http/types@0.2.0",
        "wasi:http/outgoing-handler@0.2.0",
    ]
    .into_iter()
    .map(|k| Ok((k, export_item(&virt, k)?)))
    .chain([Ok((
        "fermyon:spin-test/http-helper",
        export_item(&virt, "fermyon:spin-wasi-virt/http-helper")?,
    ))]);

    // Collect args and instantiate the `test` component
    let test_args = router_args
        .chain(virt_args)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let test_args = test_args
        .iter()
        .map(|(k, v)| (*k, v as &dyn composition::InstantiationArg));
    composition
        .instantiate("test", &test_component.bytes, test_args)
        .with_context(|| {
            format!(
                "failed to instantiate test component '{}'",
                test_component.path.display()
            )
        })
}

fn instantiate_router(
    composition: &Composition,
    virt: &composition::Instance,
    app: composition::Instance,
) -> anyhow::Result<composition::Instance> {
    // Get access to the `http/types` and `http-helper` exports
    let http_types = export_instance(virt, "wasi:http/types@0.2.0")?;
    let http_helper = export_instance(virt, "fermyon:spin-wasi-virt/http-helper")?;

    let router_args = [
        ("wasi:http/types@0.2.0", virt),
        ("wasi:io/error@0.2.0", virt),
        ("wasi:io/streams@0.2.0", virt),
        ("wasi:io/poll@0.2.0", virt),
        ("wasi:cli/stdout@0.2.0", virt),
        ("set-component-id", virt),
        ("wasi:http/incoming-handler@0.2.0", &app),
        ("outgoing-request", &http_types),
        ("incoming-request", &http_types),
        ("incoming-body", &http_types),
        ("new-request", &http_helper),
    ]
    .into_iter()
    .map(|(k, v)| Ok((k, export_item(v, k)?)))
    .collect::<anyhow::Result<Vec<_>>>()?;

    let router_args = router_args
        .iter()
        .map(|(k, v)| (*k, v as &dyn composition::InstantiationArg));
    let router = composition
        .instantiate("router", ROUTER, router_args)
        .context("failed to instantiate router")?;
    Ok(router)
}

fn instantiate_app(
    composition: &Composition,
    app_component: Component,
    virt: &composition::Instance,
) -> anyhow::Result<composition::Instance, anyhow::Error> {
    let app_args = [
        "fermyon:spin/key-value@2.0.0",
        "fermyon:spin/llm@2.0.0",
        "fermyon:spin/redis@2.0.0",
        "fermyon:spin/rdbms-types@2.0.0",
        "fermyon:spin/mysql@2.0.0",
        "fermyon:spin/postgres@2.0.0",
        "fermyon:spin/sqlite@2.0.0",
        "fermyon:spin/mqtt@2.0.0",
        "fermyon:spin/variables@2.0.0",
        "wasi:io/error@0.2.0",
        "wasi:io/streams@0.2.0",
        "wasi:io/poll@0.2.0",
        "wasi:clocks/monotonic-clock@0.2.0",
        "wasi:clocks/wall-clock@0.2.0",
        "wasi:random/random@0.2.0",
        "wasi:random/insecure@0.2.0",
        "wasi:random/insecure-seed@0.2.0",
        "wasi:cli/terminal-input@0.2.0",
        "wasi:cli/terminal-output@0.2.0",
        "wasi:cli/terminal-stdin@0.2.0",
        "wasi:cli/terminal-stdout@0.2.0",
        "wasi:cli/terminal-stderr@0.2.0",
        "wasi:cli/environment@0.2.0",
        "wasi:cli/exit@0.2.0",
        "wasi:cli/environment@0.2.0",
        "wasi:cli/exit@0.2.0",
        "wasi:cli/stdin@0.2.0",
        "wasi:cli/stderr@0.2.0",
        "wasi:cli/stdout@0.2.0",
        "wasi:filesystem/types@0.2.0",
        "wasi:filesystem/preopens@0.2.0",
        "wasi:sockets/instance-network@0.2.0",
        "wasi:sockets/network@0.2.0",
        "wasi:sockets/udp@0.2.0",
        "wasi:sockets/udp-create-socket@0.2.0",
        "wasi:sockets/tcp@0.2.0",
        "wasi:sockets/tcp-create-socket@0.2.0",
        "wasi:sockets/ip-name-lookup@0.2.0",
        "wasi:http/outgoing-handler@0.2.0",
        "wasi:http/types@0.2.0",
    ]
    .into_iter()
    .map(|k| Ok((k, export_item(virt, k)?)))
    .collect::<anyhow::Result<Vec<_>>>()?;

    let app_args = app_args
        .iter()
        .map(|(k, v)| (*k, v as &dyn composition::InstantiationArg));
    let app = composition
        .instantiate("app", &app_component.bytes, app_args)
        .context("failed to instantiate Spin app")?;
    Ok(app)
}

/// Instantiate the `virt` component and export the `fs-handler` instance
fn instantiate_virt(composition: &Composition) -> anyhow::Result<composition::Instance> {
    let virt = composition
        .instantiate("virt", SPIN_TEST_VIRT, Vec::new())
        .context("fatal error: could not instantiate spin-test-virt")?;
    let fs_handler = export_item(&virt, "fermyon:spin-wasi-virt/fs-handler")?;
    composition
        .export(fs_handler, "fermyon:spin-wasi-virt/fs-handler")
        .context("fatal error: could not export fs-handler from spin-test-virt")?;
    Ok(virt)
}

/// Export an instance from an instance
fn export_instance(
    instance: &composition::Instance,
    name: &str,
) -> anyhow::Result<composition::Instance> {
    export_item(instance, name)?.as_instance().with_context(|| {
        format!(
            "internal error: export `{name}` in `{}` is not an instance",
            instance.name()
        )
    })
}

/// Export an item from an instance
fn export_item(
    instance: &composition::Instance,
    name: &str,
) -> anyhow::Result<composition::InstanceExport> {
    instance
        .export(name)
        .with_context(|| format!("failed to export '{name}' from {}", instance.name()))?
        .with_context(|| format!("{} does not export '{name}'", instance.name()))
}

/// Represents the target type of the test component.
#[derive(Debug)]
pub enum TestTarget {
    /// The `AdHoc` target indicates the test component contains a set
    /// of exports prefixed with "spin-test-*" that should be called
    /// to execute a suite of tests.
    AdHoc {
        /// The set of exports prefixed with `spin-test-*`.
        exports: HashSet<String>,
    },
    /// The `TestWorld` target indicates the test component exports
    /// a singular `run` export that takes a test name as an argument.
    TestWorld { tests: HashSet<String> },
}

impl TestTarget {
    pub const SPIN_TEST_NAME_PREFIX: &'static str = "spin-test-";
    const RUN_EXPORT: &'static str = "run";

    /// Determine the test target type from a test component.
    pub fn from_component(test: &Component) -> anyhow::Result<Self> {
        let decoded = wit_component::decode(&test.bytes)
            .context("failed to decode test component's wit package")?;
        let resolve = decoded.resolve();
        let package = decoded.package();

        let world_id = resolve.select_world(package, None)?;
        let world = &resolve.worlds[world_id];

        let mut exports = HashSet::new();
        let mut seen_run = false;

        for (export_key, _export_item) in world.exports.iter() {
            match resolve.name_world_key(export_key) {
                name if name.starts_with(Self::SPIN_TEST_NAME_PREFIX) => {
                    // TODO: ensure export_item is a freestanding function?
                    assert!(exports.insert(name));
                }
                name if name == Self::RUN_EXPORT => seen_run = true,
                _ => {}
            }
        }

        // Ensure we are either dealing with a test component that exports `run` OR
        // exports the specially prefixed ad-hoc test exports.
        if seen_run {
            if !exports.is_empty() {
                anyhow::bail!("expected test component to export either test functions `spin-test-*` or a `run` function, but it exported both");
            }

            let tests = get_tests_list(test, world, resolve)
                .context("failed to read list of tests from the test component")?;
            Ok(TestTarget::TestWorld {
                tests: tests.into_iter().collect(),
            })
        } else {
            if exports.is_empty() {
                anyhow::bail!("expected test component to export either test functions `spin-test-*` or a `run` function, but it exported neither");
            }
            Ok(TestTarget::AdHoc { exports })
        }
    }
}

/// Get the list of tests from a test component
fn get_tests_list(
    test_component: &Component,
    world: &wit_parser::World,
    resolve: &wit_parser::Resolve,
) -> Result<Vec<String>, anyhow::Error> {
    struct TestComponentData {
        table: wasmtime_wasi::ResourceTable,
        ctx: wasmtime_wasi::WasiCtx,
    }

    impl wasmtime_wasi::WasiView for TestComponentData {
        fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
            &mut self.table
        }
        fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
            &mut self.ctx
        }
    }

    let engine = wasmtime::Engine::default();
    let mut store = wasmtime::Store::new(
        &engine,
        TestComponentData {
            table: wasmtime_wasi::ResourceTable::default(),
            ctx: wasmtime_wasi::WasiCtxBuilder::new()
                .inherit_stdout()
                .inherit_stderr()
                .build(),
        },
    );
    let component = wasmtime::component::Component::new(&engine, &test_component.bytes)
        .context("test component was an invalid Wasm component")?;

    // Configure the linker including stubbing out all non-wasi imports
    let mut linker = wasmtime::component::Linker::<TestComponentData>::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)
        .context("failed to link to wasi to the test component")?;
    stub_imports(&mut linker, world, resolve).context("failed to stub test component imports")?;

    // Instantiate the test component and call the `list-tests` export
    let instance = linker
        .instantiate(&mut store, &component)
        .context("failed to instantiate test component")?;
    let func = instance
        .get_typed_func::<(), (Vec<String>,)>(&mut store, "list-tests")
        .context("test component is missing the `list-tests` export")?;
    let (tests,) = func
        .call(&mut store, ())
        .context("failed to call test component's `list-tests` export")?;
    Ok(tests)
}

/// Stub out all imports with functions that panic
fn stub_imports<T>(
    linker: &mut wasmtime::component::Linker<T>,
    world: &wit_parser::World,
    resolve: &wit_parser::Resolve,
) -> anyhow::Result<()> {
    for (import_name, import) in world.imports.iter() {
        let import_name = resolve.name_world_key(import_name);
        match import {
            wit_parser::WorldItem::Interface(i) => {
                let interface = resolve.interfaces.get(*i).unwrap();
                let mut root = linker.root();
                let Ok(mut instance) = root.instance(&import_name) else {
                    // We've already seen this instance, skip it
                    continue;
                };
                for (_, f) in interface.functions.iter() {
                    let import_name = import_name.clone();
                    let func_name = f.name.clone();
                    instance
                        .func_new(&f.name, move |_ctx, _args, _rets| {
                            panic!("unexpected call to `{import_name}/{func_name}`")
                        })
                        .with_context(|| format!("failed to link function '{}'", f.name))?;
                }
                for (name, t) in &interface.types {
                    let t = resolve.types.get(*t).unwrap();
                    if let wit_parser::TypeDefKind::Resource = &t.kind {
                        let ty = wasmtime::component::ResourceType::host::<()>();
                        instance.resource(name, ty, |_, _| Ok(())).unwrap();
                    }
                }
            }
            wit_parser::WorldItem::Function(f) => {
                let func_name = f.name.clone();
                linker
                    .root()
                    .func_new(&f.name, move |_ctx, _args, _rets| {
                        panic!("unexpected call to `{func_name}`");
                    })
                    .with_context(|| format!("failed to link function '{}'", f.name))?;
            }
            wit_parser::WorldItem::Type(_) => {}
        }
    }
    Ok(())
}
