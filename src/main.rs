use std::path::PathBuf;

use anyhow::Context as _;
use clap::Parser;
use owo_colors::OwoColorize as _;
use spin_test::{runtime::TestInvocation, Component, ManifestInformation, TestTarget};

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
    /// Virtualize a Spin application
    Virtualize(Virtualize),
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
        Subcommand::Virtualize(v) => v.exec(),
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
        let test_path = manifest
            .test_path()
            .context("failed to read the path to the test component from the spin.toml manifest")?;

        let app_component = Component::from_file(manifest.app_source()?.into())
            .context("failed to read app component")?;
        let test_component = Component::from_file(test_path.to_owned())
            .with_context(|| format!("failed to read test component '{}'", test_path.display()))?;
        let test_target = TestTarget::from_component(&test_component).with_context(|| {
            format!(
                "failed to determine how to run the tests from test component '{}'",
                test_path.display()
            )
        })?;
        let encoded =
            spin_test::perform_composition(app_component, test_component, &test_target)
                .context("failed to compose Spin app, test, and virtualized Spin environment")?;

        let tests = run_tests(test_target, encoded, manifest)?;
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
    test_target: spin_test::TestTarget,
    encoded: Vec<u8>,
    manifest: ManifestInformation,
) -> anyhow::Result<Vec<libtest_mimic::Trial>> {
    let encoded = std::sync::Arc::new(encoded);

    let tests: Vec<_> = match test_target {
        spin_test::TestTarget::AdHoc { exports } => exports
            .into_iter()
            .map(|test_export| {
                let test_name = test_export
                    .strip_prefix(TestTarget::SPIN_TEST_NAME_PREFIX)
                    .unwrap()
                    .to_owned();
                (test_name.clone(), TestInvocation::Export(test_export))
            })
            .collect(),
        spin_test::TestTarget::TestWorld { tests } => tests
            .into_iter()
            .map(|test| (test.clone(), TestInvocation::RunArgument(test)))
            .collect(),
    };
    let trials = tests
        .into_iter()
        .map(|(test_name, test)| {
            let manifest = manifest.clone();
            let encoded = encoded.clone();

            libtest_mimic::Trial::test(test_name, move || {
                let mut runtime = spin_test::runtime::Runtime::instantiate(manifest, &encoded)?;

                Ok(runtime.run(test).map_err(FullError::from)?)
            })
        })
        .collect();

    Ok(trials)
}

#[derive(clap::Parser)]
struct Virtualize {
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

impl Virtualize {
    fn exec(self) -> anyhow::Result<()> {
        let manifest = ManifestInformation::resolve(self.app_source)?;
        let app_component = Component::from_file(manifest.app_source()?.into())
            .context("failed to read app component")?;
        let encoded =
            spin_test::virtualize_app(app_component).context("failed to virtualize app")?;
        std::fs::write("virtualized.wasm", encoded)
            .context("failed to write virtualized app to disk")?;
        println!("Successfully virtualized app to virtualized.wasm");
        Ok(())
    }
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
