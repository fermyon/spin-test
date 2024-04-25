use std::path::PathBuf;

use anyhow::{bail, Context};
use clap::Parser;
use owo_colors::OwoColorize as _;
use spin_test::{Component, TestTarget};

#[derive(clap::Parser)]
#[command(version, about)]
/// Run tests against a Spin application.
///
/// By default `spin-test` will invoke the `run` subcommand.
struct Cli {
    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(clap::Parser)]
enum Subcommand {
    /// Run a test suite against a Spin application
    Run(Run),
}

impl Default for Subcommand {
    fn default() -> Self {
        Self::Run(Run::parse())
    }
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
    match Cli::parse().subcommand.unwrap_or_default() {
        Subcommand::Run(r) => r.exec(),
    }
}

#[derive(clap::Parser)]
struct Run {
    /// The manifest (spin.toml) file for the application under test.
    ///
    /// This may be a manifest file or a directory containing a spin.toml file.
    #[clap(
        name = "APP_MANIFEST_FILE",
        short = 'f',
        long = "from",
        alias = "file",
        default_value = spin_common::paths::DEFAULT_MANIFEST_FILE,
    )]
    pub app_source: PathBuf,
}

impl Run {
    fn exec(self) -> anyhow::Result<()> {
        let manifest_path = spin_common::paths::resolve_manifest_file_path(self.app_source)
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
        let component = manifest
            .components
            .values()
            .next()
            .context("spin.toml did not contain any components")?;
        let app_path = match &component.source {
            spin_manifest::schema::v2::ComponentSource::Local(path) => path,
            spin_manifest::schema::v2::ComponentSource::Remote { .. } => {
                bail!("components with remote sources are not yet supported by `spin-test`")
            }
        };
        let spin_test_config = component
            .tool
            .get("spin-test")
            .context("component did not have a `spin-test` tool configuration")?;

        if let Some(build) = spin_test_config.get("build").and_then(|b| b.as_str()) {
            let dir = spin_test_config.get("dir").and_then(|d| d.as_str());
            let mut cmd = std::process::Command::new("/bin/sh");
            if let Some(dir) = dir {
                cmd.current_dir(dir);
            }
            cmd.args(["-c", build])
                .status()
                .context("failed to build component")?;
        }
        let test_source = spin_test_config
            .get("source")
            .context("component did not have a `spin-test.source` configuration")?
            .as_str()
            .context("component `spin-test.source` was not a string")?;
        let test_path = std::path::Path::new(test_source);
        let test_name = test_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("test");

        let (encoded, test_target) = spin_test::encode_composition(
            Component::from_file(app_path.into())?,
            Component::from_file(test_path.to_owned())?,
        )
        .context("failed to compose Spin app, test, and virtualized Spin environment")?;

        let tests = run_tests(test_name, test_target, raw_manifest, encoded)?;
        let _ = libtest_mimic::run(&libtest_mimic::Arguments::default(), tests);

        Ok(())
    }
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
    let encoded = std::sync::Arc::new(encoded);

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
