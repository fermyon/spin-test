mod composition;
pub mod runtime;

use std::{collections::HashSet, path::PathBuf};

use anyhow::Context as _;
pub use composition::Composition;

/// The built `spin-test-virt` component
const SPIN_TEST_VIRT: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-wasi/release/spin_test_virt.wasm"
));
/// The built `spin-wasi-virt` component
const SPIN_WASI_VIRT: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-wasi/release/spin_wasi_virt.wasm"
));
/// The built `router` component
const ROUTER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wasm32-wasi/release/router.wasm"));
/// The wit package for `spin-test`
const WIT: &str = concat!(env!("OUT_DIR"), "/wit");

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

    let composition = Composition::new();

    // Get definition of the `fermyon:spin-test/http-helper` instance from the wit package
    // The instance is buried in an `http-helper` component export.
    let mut resolve = wit_parser::Resolve::new();
    let (pkg, _) = resolve
        .push_dir(std::path::Path::new(WIT))
        .expect("failed to push host-wit directory");
    let wit_bytes = wit_component::encode(Some(true), &resolve, pkg).unwrap();
    let wit = composition.register_package("wit", &wit_bytes)?;
    let http_helper = wit
        .get_export("http-helper")
        .expect("internal error: no 'http-helper' component export found in wit package'")
        .as_component()
        .expect("'http-helper' export was not a component");
    let http_helper = http_helper
        .get_export("fermyon:spin-test/http-helper")
        .expect(
            "internal error: `fermyon:spin-test/http-helper` not found in 'http-helper' component",
        )
        .as_instance()
        .expect("internal error: `fermyon:spin-test/http-helper` is not an instance");

    // Import the `fermyon:spin-test/http-helper` instance into the composition
    let http_helper = composition.import_instance("fermyon:spin-test/http-helper", http_helper)?;

    // Instantiate the `spin-test-virt` component with the `futurize-response` export from the `http-helper` instance
    let futurize_response = http_helper
        .export("futurize-response")?
        .expect("internal error: `futurize-response` not found");
    let virt_args = vec![("futurize-response", Box::new(futurize_response) as Box<_>)];
    let virt = composition
        .instantiate("virt", SPIN_TEST_VIRT, virt_args)
        .context("fatal error: could not instantiate spin-test-virt")?;

    let wasi_virt = composition
        .instantiate("wasi-virt", SPIN_WASI_VIRT, Vec::new())
        .context("fatal error: could not instantiate spin-wasi-virt")?;

    // Instantiate the `app` component with various exports from `spin-test-virt` instance
    let app_args = [
        "fermyon:spin/key-value@2.0.0",
        "fermyon:spin/llm@2.0.0",
        "fermyon:spin/redis@2.0.0",
        // TODO: pass this once `spin-test-virt` exports it
        // "fermyon:spin/rdms-types@2.0.0",
        "fermyon:spin/mysql@2.0.0",
        "fermyon:spin/postgres@2.0.0",
        "fermyon:spin/sqlite@2.0.0",
        "fermyon:spin/mqtt@2.0.0",
        "fermyon:spin/variables@2.0.0",
        "wasi:http/outgoing-handler@0.2.0",
    ]
    .into_iter()
    .map(|k| {
        Ok((
            k,
            Box::new(
                virt.export(k)
                    .with_context(|| format!("failed to export '{k}' from `spin-test-virt`"))?
                    .with_context(|| format!("`spin-test-virt` does not export '{k}'"))?,
            ) as Box<_>,
        ))
    })
    .chain(
        [
            "wasi:cli/environment@0.2.0",
            "wasi:cli/exit@0.2.0",
            "wasi:filesystem/types@0.2.0",
            "wasi:filesystem/preopens@0.2.0",
            "wasi:cli/environment@0.2.0",
            "wasi:cli/exit@0.2.0",
            "wasi:sockets/instance-network@0.2.0",
            "wasi:sockets/network@0.2.0",
            "wasi:sockets/udp@0.2.0",
            "wasi:sockets/udp-create-socket@0.2.0",
            "wasi:sockets/tcp@0.2.0",
            "wasi:sockets/tcp-create-socket@0.2.0",
            "wasi:sockets/ip-name-lookup@0.2.0",
        ]
        .into_iter()
        .map(|k| {
            Ok((
                k,
                Box::new(
                    wasi_virt
                        .export(k)
                        .with_context(|| format!("failed to export '{k}' from `spin-wasi-virt`"))?
                        .with_context(|| format!("`spin-wasi-virt` does not export '{k}'"))?,
                ) as Box<_>,
            ))
        }),
    )
    .collect::<anyhow::Result<Vec<_>>>()?;
    let app = composition
        .instantiate("app", &app_component.bytes, app_args)
        .context("failed to instantiate Spin app")?;

    // Instantiate the `router` component with various exports from `spin-test-virt` and `app` instances
    let router_args = [
        ("set-component-id", &virt, "`spin-test-virt`"),
        ("wasi:http/incoming-handler@0.2.0", &app, "the Spin app"),
    ]
    .into_iter()
    .map(|(k, v, name)| {
        Ok((
            k,
            Box::new(
                v.export(k)
                    .with_context(|| format!("failed to export '{k}' from {name}"))?
                    .with_context(|| format!("{name} does not export '{k}'"))?,
            ) as _,
        ))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;
    let router = composition
        .instantiate("router", ROUTER, router_args)
        .context("failed to instantiate router")?;

    // Instantiate the `test` component
    let mut test_args = vec![
        ("wasi:http/incoming-handler@0.2.0", &router, "the router"),
        (
            "wasi:http/outgoing-handler@0.2.0",
            &virt,
            "`spin-test-virt`",
        ),
        ("fermyon:spin/key-value@2.0.0", &virt, "`spin-test-virt`"),
        (
            "fermyon:spin-test-virt/key-value",
            &virt,
            "`spin-test-virt`",
        ),
        ("fermyon:spin-test-virt/sqlite", &virt, "`spin-test-virt`"),
        // Needed for types
        ("fermyon:spin/sqlite@2.0.0", &virt, "`spin-test-virt`"),
        (
            "fermyon:spin-test-virt/http-handler",
            &virt,
            "`spin-test-virt`",
        ),
    ]
    .into_iter()
    .map(|(k, v, name)| {
        Ok((
            k,
            Box::new(
                v.export(k)
                    .with_context(|| format!("failed to export '{k}' from {name}"))?
                    .with_context(|| format!("{name} does not export '{k}'"))?,
            ) as Box<_>,
        ))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;
    // Explicitly import the `fermyon:spin-test/http-helper` instance into the
    // `test` component from the top-level composition's import
    test_args.push((
        "fermyon:spin-test/http-helper",
        Box::new(http_helper) as Box<_>,
    ));
    let test = composition
        .instantiate("test", &test_component.bytes, test_args)
        .with_context(|| {
            format!(
                "failed to instantiate test component '{}'",
                test_component.path.display()
            )
        })?;

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
        .encode()
        .context("failed to encode composition")?;

    Ok((bytes, test_target))
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
