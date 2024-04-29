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

    let composition = Composition::new();

    let virt = composition
        .instantiate("virt", SPIN_TEST_VIRT, Vec::new())
        .context("fatal error: could not instantiate spin-test-virt")?;

    // Instantiate the `app` component with various exports from `spin-test-virt` instance
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
    .collect::<anyhow::Result<Vec<_>>>()?;
    let app_args = app_args.iter().map(|(k, v)| (*k, &**v as _));
    let app = composition
        .instantiate("app", &app_component.bytes, app_args)
        .context("failed to instantiate Spin app")?;

    let http_types = virt
        .export("wasi:http/types@0.2.0")?
        .context("internal error: `wasi:http/types` not found in `spin-test-virt`")?
        .as_instance()
        .unwrap();
    let http_helper = virt
        .export("fermyon:spin-wasi-virt/http-helper")?
        .context("internal error: `fermyon:spin-test-virt/http-helper` not found")?
        .as_instance()
        .context("internal error: `fermyon:spin-test-virt/http-helper` is not an instance")?;
    let new_request = http_helper
        .export("new-request")?
        .expect("internal error: `new-request` not found");
    // Instantiate the `router` component with various exports from `spin-test-virt` and `app` instances
    let router_args = [
        ("set-component-id", &virt, "`spin-test-virt`"),
        ("wasi:http/incoming-handler@0.2.0", &app, "the Spin app"),
        ("outgoing-request", &http_types, "`spin-test-virt`"),
        ("incoming-request", &http_types, "`spin-test-virt`"),
        ("incoming-body", &http_types, "`spin-test-virt`"),
    ]
    .into_iter()
    .map(|(k, v, name)| {
        Ok((
            k,
            Box::new(
                v.export(k)
                    .with_context(|| format!("failed to export '{k}' from {name}"))?
                    .with_context(|| format!("{name} does not export '{k}'"))?,
            ) as Box<dyn composition::InstantiationArg>,
        ))
    })
    .chain([
        Ok(("new-request", Box::new(new_request) as _)),
        Ok(("wasi:http/types@0.2.0", Box::new(http_types.clone()) as _)),
    ])
    .collect::<anyhow::Result<Vec<_>>>()?;
    let router_args = router_args.iter().map(|(k, v)| (*k, &**v as _));
    //     .chain(io_args.iter().map(|(k, v)| (*k, v as _)));
    let router = composition
        .instantiate("router", ROUTER, router_args)
        .context("failed to instantiate router")?;

    // Instantiate the `test` component
    let test_args = vec![
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
            "fermyon:spin-wasi-virt/http-handler",
            &virt,
            "`spin-wasi-virt`",
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
            ) as Box<dyn composition::InstantiationArg>,
        ))
    })
    .chain([
        Ok((
            "wasi:http/types@0.2.0",
            Box::new(http_types.clone()) as Box<_>,
        )),
        Ok((
            "fermyon:spin-test/http-helper",
            Box::new(http_helper) as Box<_>,
        )),
    ])
    .collect::<anyhow::Result<Vec<_>>>()?;
    let wasi_imports = [
        "wasi:filesystem/types@0.2.0",
        "wasi:filesystem/preopens@0.2.0",
        "wasi:cli/stdin@0.2.0",
        "wasi:cli/stderr@0.2.0",
        "wasi:cli/stdout@0.2.0",
        "wasi:io/error@0.2.0",
        "wasi:io/streams@0.2.0",
        "wasi:io/poll@0.2.0",
    ]
    .into_iter()
    .map(|k| {
        Ok((
            k,
            Box::new(
                virt.export(k)
                    .with_context(|| format!("failed to export '{k}' from `spin-wasi-virt`"))?
                    .with_context(|| format!("`spin-wasi-virt` does not export '{k}'"))?,
            ) as Box<_>,
        ))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;
    let test_args = test_args
        .iter()
        .map(|(k, v)| (*k, &**v as &dyn composition::InstantiationArg))
        // .chain(io_args.iter().map(|(k, v)| (*k, v as _)))
        .chain(
            wasi_imports
                .iter()
                .map(|(k, v)| (*k, &**v as &dyn composition::InstantiationArg)),
        );
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
        .encode(true)
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
