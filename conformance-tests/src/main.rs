mod runtime;

use runtime::SpinTest;

// We don't use port 80 because URL parsing treats port 80 special
// and will not include it in the URL string representation which breaks the test
const HTTP_PORT: u16 = 1234;

fn main() -> anyhow::Result<()> {
    conformance_tests::run_tests(run_test)
}

fn run_test(test: conformance_tests::Test) -> Result<(), anyhow::Error> {
    let mut manifest = test_environment::manifest_template::EnvTemplate::from_file(&test.manifest)?;
    let env_config = test_environment::TestEnvironmentConfig {
        create_runtime: Box::new(|_env| {
            manifest.substitute_value("port", |port| substitution("port", port))?;
            SpinTest::new(manifest.into_contents(), test.component)
        }),
        // Services are not needed in `spin-test` since everything stays in the guest
        services_config: test_environment::services::ServicesConfig::none(),
    };
    let mut env = test_environment::TestEnvironment::up(env_config, |_| Ok(()))?;
    for precondition in &test.config.preconditions {
        match precondition {
            conformance_tests::config::Precondition::HttpEcho => {
                env.runtime_mut()
                    .set_echo_response(format!("http://localhost:{HTTP_PORT}").as_str())?;
            }
            conformance_tests::config::Precondition::TcpEcho => {}
            conformance_tests::config::Precondition::KeyValueStore(_) => {}
        }
    }
    for invocation in test.config.invocations {
        let conformance_tests::config::Invocation::Http(mut invocation) = invocation;
        invocation
            .request
            .substitute(|key, value| Ok(substitution(key, value)))?;

        invocation.run(|request| env.runtime_mut().make_http_request(request))?;
    }

    Ok(())
}

/// When encountering a magic key-value pair, substitute the value with a different value.
fn substitution(key: &str, value: &str) -> Option<String> {
    match (key, value) {
        ("port", "80") => Some(HTTP_PORT.to_string()),
        ("port", "5000") => Some(5000.to_string()),
        _ => None,
    }
}

struct StoreData {
    manifest: String,
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime::component::ResourceTable,
}

impl StoreData {
    fn new(manifest: String) -> Self {
        let ctx = wasmtime_wasi::WasiCtxBuilder::new().inherit_stdio().build();
        let table = wasmtime::component::ResourceTable::default();
        Self {
            manifest,
            ctx,
            table,
        }
    }
}

impl wasmtime_wasi::WasiView for StoreData {
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.ctx
    }
}

impl runtime::VirtualizedAppImports for StoreData {
    fn get_manifest(&mut self) -> String {
        self.manifest.clone()
    }
}
