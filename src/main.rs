use anyhow::{bail, Context};
use clap::Parser;
use owo_colors::OwoColorize as _;
use std::path::PathBuf;

mod composition;
mod runtime;

/// The built `spin-test-virt` component
const SPIN_TEST_VIRT: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-wasi/release/spin_test_virt.wasm"
));
/// The built `router` component
const ROUTER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wasm32-wasi/release/router.wasm"));

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to test wasm
    test_path: PathBuf,
}

fn main() {
    env_logger::init();
    if let Err(e) = _main() {
        eprintln!(
            "{error}: {e}",
            error = "error".if_supports_color(owo_colors::Stream::Stderr, |text| {
                text.style(owo_colors::Style::new().red().bold())
            }),
        );
        print_error_chain(e);
        std::process::exit(1);
    }
}

fn _main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let test_path = cli.test_path;
    let manifest_path =
        spin_common::paths::resolve_manifest_file_path(spin_common::paths::DEFAULT_MANIFEST_FILE)
            .context("failed to find spin.toml manifest file")?;
    let raw_manifest = std::fs::read_to_string(&manifest_path).with_context(|| {
        format!(
            "failed to read spin.toml manifest at {}",
            manifest_path.display()
        )
    })?;
    let manifest = spin_manifest::manifest_from_str(&raw_manifest).with_context(|| {
        format!(
            "failed to deserialize spin.toml manifest at {}",
            manifest_path.display()
        )
    })?;
    if manifest.components.len() > 1 {
        bail!("Spin applications with more than one component are not yet supported by `spin-test`")
    }
    let app_path = match &manifest
        .components
        .values()
        .next()
        .context("spin.toml did not contain any components")?
        .source
    {
        spin_manifest::schema::v2::ComponentSource::Local(path) => path,
        spin_manifest::schema::v2::ComponentSource::Remote { .. } => {
            bail!("components with remote sources are not yet supported by `spin-test`")
        }
    };

    let test = std::fs::read(&test_path).with_context(|| {
        format!(
            "failed to read test wasm binary at '{}'",
            test_path.display()
        )
    })?;
    let app = std::fs::read(app_path)
        .with_context(|| format!("failed to read app source at '{app_path}'"))?;
    let app = spin_componentize::componentize_if_necessary(&app)
        .context("failed to turn app module into a component")?
        .into_owned();

    let encoded = encode_composition(
        Component {
            bytes: app,
            path: PathBuf::from(app_path),
        },
        Component {
            bytes: test,
            path: test_path.clone(),
        },
    )
    .context("failed to compose Spin app, test, and virtualized Spin environment")?;

    let mut runtime = runtime::Runtime::instantiate(raw_manifest, &encoded)
        .context("failed to create `spin-test` runtime")?;
    let tests = vec![libtest_mimic::Trial::test(
        test_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("test"),
        move || Ok(runtime.run()?),
    )];
    let _ = libtest_mimic::run(&libtest_mimic::Arguments::default(), tests);
    Ok(())
}

struct Component {
    bytes: Vec<u8>,
    path: PathBuf,
}

fn encode_composition(
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

fn print_error_chain(err: anyhow::Error) {
    if let Some(cause) = err.source() {
        let is_multiple = cause.source().is_some();
        eprintln!("\nCaused by:");
        for (i, err) in err.chain().skip(1).enumerate() {
            if is_multiple {
                eprintln!("{i:>4}: {}", err)
            } else {
                eprintln!("      {}", err)
            }
        }
    }
}
