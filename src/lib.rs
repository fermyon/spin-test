mod composition;
pub mod runtime;

use std::path::PathBuf;

use anyhow::Context as _;

/// The built `spin-test-virt` component
const SPIN_TEST_VIRT: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-wasi/release/spin_test_virt.wasm"
));
/// The built `router` component
const ROUTER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wasm32-wasi/release/router.wasm"));

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
            .context("failed to turn app module into a component")?
            .into_owned();
        Ok(Self { bytes, path })
    }
}

/// Encode a composition of an app component and a test component
pub fn encode_composition(
    app_component: Component,
    test_component: Component,
) -> anyhow::Result<Vec<u8>> {
    let composition = composition::Composition::new();
    let virt = composition
        .instantiate("virt", SPIN_TEST_VIRT, Vec::new())
        .context("fatal error: could not instantiate spin-test-virt")?;

    let app_args = [
        "fermyon:spin/key-value@2.0.0",
        "fermyon:spin/llm@2.0.0",
        "fermyon:spin/redis@2.0.0",
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
            virt.export(k)
                .with_context(|| format!("failed to export '{k}' from `spin-test-virt`"))?
                .with_context(|| format!("`spin-test-virt` does not export '{k}'"))?,
        ))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;
    let app = composition
        .instantiate("app", &app_component.bytes, app_args)
        .context("failed to instantiate Spin app")?;

    let router_args = [
        ("set-component-id", &virt, "`spin-test-virt`"),
        ("wasi:http/incoming-handler@0.2.0", &app, "the Spin app"),
    ]
    .into_iter()
    .map(|(k, v, name)| {
        Ok((
            k,
            v.export(k)
                .with_context(|| format!("failed to export '{k}' from {name}"))?
                .with_context(|| format!("{name} does not export '{k}'"))?,
        ))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;
    let router = composition
        .instantiate("router", ROUTER, router_args)
        .context("failed to instantiate router")?;

    let test_args = vec![
        ("wasi:http/incoming-handler@0.2.0", &router, "the router"),
        (
            "wasi:http/outgoing-handler@0.2.0",
            &virt,
            "`spin-test-virt`",
        ),
        ("fermyon:spin/key-value@2.0.0", &virt, "`spin-test-virt`"),
        (
            "fermyon:spin-test-virt/key-value-calls",
            &virt,
            "`spin-test-virt`",
        ),
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
            v.export(k)
                .with_context(|| format!("failed to export '{k}' from {name}"))?
                .with_context(|| format!("{name} does not export '{k}'"))?,
        ))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;
    let test = composition
        .instantiate("test", &test_component.bytes, test_args)
        .with_context(|| {
            format!(
                "failed to instantiate test component '{}'",
                test_component.path.display()
            )
        })?;
    let export = test
        .export("run")
        .context("failed to export 'run' function from test component")?
        .context("test component must contain 'run' function but it did not")?;

    composition
        .export(export, "run")
        .context("failed to export 'run' function from composition")?;
    composition.encode().context("failed to encode composition")
}
