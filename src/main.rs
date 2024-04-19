use anyhow::{bail, Context};
use clap::Parser;
use owo_colors::OwoColorize as _;
use spin_test::{Component, TestTarget};
use std::path::PathBuf;

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

    let test_name = test_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("test");

    let (encoded, test_target) = spin_test::encode_composition(
        Component::from_file(app_path.into())?,
        Component::from_file(test_path.clone())?,
    )
    .context("failed to compose Spin app, test, and virtualized Spin environment")?;

    let tests = run_tests(test_name, test_target, raw_manifest, encoded)?;
    let _ = libtest_mimic::run(&libtest_mimic::Arguments::default(), tests);

    Ok(())
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

fn run_tests(
    test_name: &str,
    test_target: spin_test::TestTarget,
    raw_manifest: String,
    encoded: Vec<u8>,
) -> anyhow::Result<Vec<libtest_mimic::Trial>> {
    let mut trials = vec![];

    match test_target {
        spin_test::TestTarget::AdHoc { exports } => {
            for test_export in exports {
                let test_name = test_export
                    .strip_prefix(TestTarget::SPIN_TEST_NAME_PREFIX)
                    .unwrap()
                    .to_owned();
                let raw_manifest = raw_manifest.clone();
                let encoded = encoded.clone();

                trials.push(libtest_mimic::Trial::test(test_name, move || {
                    let mut runtime =
                        spin_test::runtime::Runtime::instantiate(raw_manifest, &encoded)
                            .context("failed to create `spin-test` runtime")?;

                    Ok(runtime.run(Some(&test_export))?)
                }));
            }
        }
        spin_test::TestTarget::TestWorld => {
            trials.push(libtest_mimic::Trial::test(test_name, move || {
                let mut runtime = spin_test::runtime::Runtime::instantiate(raw_manifest, &encoded)
                    .context("failed to create `spin-test` runtime")?;

                Ok(runtime.run(None)?)
            }));
        }
    }

    Ok(trials)
}
