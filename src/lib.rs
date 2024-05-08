mod composition;
pub mod runtime;

use std::{collections::HashSet, path::PathBuf};

use anyhow::Context;
pub use composition::Composition;

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
pub fn encode_composition(
    app_component: Component,
    test_component: Component,
) -> anyhow::Result<(Vec<u8>, TestTarget)> {
    let test_target = TestTarget::from_component(&test_component)?;
    let test_target = test_target;

    let composition = Composition::new();

    let virt = composition
        .instantiate("virt", SPIN_TEST_VIRT, Vec::new())
        .context("fatal error: could not instantiate spin-test-virt")?;

    // Instantiate the `app` component with various exports from `spin-test-virt` instance
    let app = instantiate_app(&composition, app_component, &virt)?;

    // Instantiate the `router` component
    let router = instantiate_router(&composition, &virt, app)?;

    // Instantiate the `test` component
    let test = instantiate_test(&composition, test_component, router, virt)?;

    match &test_target {
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
        TestTarget::TestWorld => {
            let export = test
                .export("run")
                .context("failed to export 'run' function from test component")?
                .context("test component must contain 'run' function but it did not")?;

            composition
                .export(export, "run")
                .context("failed to export 'run' function from composition")?;
        }
    }

    let bytes = composition
        .encode(true)
        .context("failed to encode composition")?;

    Ok((bytes, test_target))
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
        "wasi:http/outgoing-handler@0.2.0",
        "wasi:filesystem/types@0.2.0",
        "wasi:filesystem/preopens@0.2.0",
        "wasi:cli/stdin@0.2.0",
        "wasi:cli/stderr@0.2.0",
        "wasi:cli/stdout@0.2.0",
        "wasi:io/error@0.2.0",
        "wasi:io/streams@0.2.0",
        "wasi:io/poll@0.2.0",
        "wasi:http/types@0.2.0",
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
        "wasi:http/outgoing-handler@0.2.0",
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
        "wasi:sockets/instance-network@0.2.0",
        "wasi:sockets/network@0.2.0",
        "wasi:sockets/udp@0.2.0",
        "wasi:sockets/udp-create-socket@0.2.0",
        "wasi:sockets/tcp@0.2.0",
        "wasi:sockets/tcp-create-socket@0.2.0",
        "wasi:sockets/ip-name-lookup@0.2.0",
        "wasi:filesystem/types@0.2.0",
        "wasi:filesystem/preopens@0.2.0",
        "wasi:cli/stdin@0.2.0",
        "wasi:cli/stderr@0.2.0",
        "wasi:cli/stdout@0.2.0",
        "wasi:http/types@0.2.0",
        "wasi:io/error@0.2.0",
        "wasi:io/streams@0.2.0",
        "wasi:io/poll@0.2.0",
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
    /// a singular `run` export to execute the test.
    TestWorld,
}

impl TestTarget {
    pub const SPIN_TEST_NAME_PREFIX: &'static str = "spin-test-";
    const RUN_EXPORT: &'static str = "run";

    fn from_component(test: &Component) -> anyhow::Result<Self> {
        let decoded = wit_component::decode(&test.bytes).context("failed to decode component")?;
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
                anyhow::bail!("expected ad hoc `spin-test-*` exports or `run`; found both");
            }
            Ok(TestTarget::TestWorld)
        } else {
            if exports.is_empty() {
                anyhow::bail!("expected ad hoc `spin-test-*` exports or `run`; found neither");
            }
            Ok(TestTarget::AdHoc { exports })
        }
    }
}
