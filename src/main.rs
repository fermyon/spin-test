use std::path::PathBuf;

use anyhow::Context as _;
use clap::Parser;
use owo_colors::OwoColorize as _;
use spin_test::{Component, ManifestInformation, TestTarget};

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
        let manifest = ManifestInformation::resolve(self.app_source)?;
        if let Some(build) = manifest.build_info()? {
            build.exec()?;
        }
        let test_path = manifest.test_path()?;
        let test_name = test_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("test")
            .to_owned();

        let (encoded, test_target) = spin_test::encode_composition(
            Component::from_file(manifest.app_source()?.into())?,
            Component::from_file(test_path.to_owned())?,
        )
        .context("failed to compose Spin app, test, and virtualized Spin environment")?;

        let tests = run_tests(&test_name, test_target, encoded, manifest)?;
        libtest_mimic::run(&libtest_mimic::Arguments::default(), tests).exit();
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
    encoded: Vec<u8>,
    manifest: ManifestInformation,
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
                let manifest = manifest.clone();
                let encoded = encoded.clone();

                trials.push(libtest_mimic::Trial::test(test_name, move || {
                    let mut runtime = spin_test::runtime::Runtime::instantiate(manifest, &encoded)?;

                    Ok(runtime.run(Some(&test_export)).map_err(FullError::from)?)
                }));
            }
        }
        spin_test::TestTarget::TestWorld => {
            trials.push(libtest_mimic::Trial::test(test_name, move || {
                let mut runtime = spin_test::runtime::Runtime::instantiate(manifest, &encoded)
                    .context("failed to create `spin-test` runtime")?;

                Ok(runtime.run(None).map_err(FullError::from)?)
            }));
        }
    }

    Ok(trials)
}

struct FullError {
    error: anyhow::Error,
}

impl From<anyhow::Error> for FullError {
    fn from(error: anyhow::Error) -> Self {
        Self { error }
    }
}

impl std::fmt::Display for FullError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)?;
        for cause in self.error.chain().skip(1) {
            write!(f, "\nCaused by: {}", cause)?;
        }
        Ok(())
    }
}
