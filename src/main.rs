use clap::Parser;
use std::path::PathBuf;

mod composition;
mod runtime;

const SPIN_TEST_VIRT: &[u8] = include_bytes!("../example/deps/fermyon/spin-test-virt.wasm");
const WASI_VIRT: &[u8] = include_bytes!("../example/deps/wasi/virt.wasm");
const ROUTER: &[u8] = include_bytes!("../example/deps/fermyon/router.wasm");

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to test wasm
    test_path: PathBuf,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();
    let test_path = cli.test_path;
    let manifest_path =
        spin_common::paths::resolve_manifest_file_path(spin_common::paths::DEFAULT_MANIFEST_FILE)
            .unwrap();
    let raw_manifest = std::fs::read_to_string(&manifest_path).unwrap();
    let manifest = spin_manifest::manifest_from_str(&raw_manifest).unwrap();
    let app_path = match &manifest.components.first().as_ref().unwrap().1.source {
        spin_manifest::schema::v2::ComponentSource::Local(path) => path,
        spin_manifest::schema::v2::ComponentSource::Remote { .. } => {
            todo!("handle remote component sources")
        }
    };

    let test = std::fs::read(&test_path).unwrap();
    let app = std::fs::read(app_path).unwrap();
    let app = spin_componentize::componentize_if_necessary(&app)
        .unwrap()
        .into_owned();

    let encoded = encode_composition(app, test);

    let mut runtime = runtime::Runtime::new(raw_manifest, &encoded);
    let tests = vec![libtest_mimic::Trial::test(
        test_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("test"),
        move || Ok(runtime.run()?),
    )];
    let _ = libtest_mimic::run(&libtest_mimic::Arguments::default(), tests);
}

fn encode_composition(app: Vec<u8>, test: Vec<u8>) -> Vec<u8> {
    let composition = composition::Composition::new();
    let virt = composition
        .instantiate("virt", SPIN_TEST_VIRT, Vec::new())
        .unwrap();
    let _wasi_virt = composition
        .instantiate("wasi_virt", WASI_VIRT, Vec::new())
        .unwrap();

    let app_args = [
        ("fermyon:spin/key-value@2.0.0", &virt),
        ("fermyon:spin/llm@2.0.0", &virt),
        ("fermyon:spin/redis@2.0.0", &virt),
        ("fermyon:spin/mysql@2.0.0", &virt),
        ("fermyon:spin/postgres@2.0.0", &virt),
        ("fermyon:spin/sqlite@2.0.0", &virt),
        ("fermyon:spin/mqtt@2.0.0", &virt),
        ("fermyon:spin/variables@2.0.0", &virt),
        ("wasi:http/outgoing-handler@0.2.0", &virt),
        // Don't stub environment yet as this messes with Python
        // ("wasi:cli/environment@0.2.0", &wasi_virt),
    ]
    .into_iter()
    .map(|(k, v)| (k, v.export(k).unwrap().unwrap()));
    let app = composition.instantiate("app", &app, app_args).unwrap();

    let router_args = [
        ("set-component-id", &virt),
        ("wasi:http/incoming-handler@0.2.0", &app),
    ]
    .into_iter()
    .map(|(k, v)| (k, v.export(k).unwrap().unwrap()));
    let router = composition
        .instantiate("router", ROUTER, router_args)
        .unwrap();

    let test_args = vec![
        ("wasi:http/incoming-handler@0.2.0", &router),
        ("wasi:http/outgoing-handler@0.2.0", &virt),
        ("fermyon:spin/key-value@2.0.0", &virt),
        ("fermyon:spin-test-virt/key-value-calls", &virt),
        ("fermyon:spin-test-virt/http-handler", &virt),
    ]
    .into_iter()
    .map(|(k, v)| (k, v.export(k).unwrap().unwrap()));
    let test = composition.instantiate("test", &test, test_args).unwrap();
    let export = test.export("run").unwrap().unwrap();

    composition.export(export, "run").unwrap();
    composition.encode().unwrap()
}
