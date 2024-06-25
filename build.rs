use std::{env, process};

fn main() {
    check_cargo_component_installed();
    cargo_component_build(
        "crates/router",
        Target::Wasm32UnknownUnknown,
        std::iter::empty(),
    );
    let wasi_env = ensure_wasi_sdk();
    cargo_component_build(
        "crates/spin-test-virt",
        Target::Wasm32Wasi,
        wasi_env.iter().map(|(k, v)| (*k, v.as_str())),
    );
}

fn ensure_wasi_sdk() -> Vec<(&'static str, String)> {
    let Ok(wasi_sdk_path_string) = std::env::var("WASI_SDK_PATH") else {
        panic!("WASI_SDK_PATH env variable is not set.");
    };
    let wasi_sdk_path = &std::path::Path::new(&wasi_sdk_path_string);
    if !wasi_sdk_path.exists() {
        panic!(
            "WASI_SDK_PATH is set to a non-existent path: {}",
            wasi_sdk_path.display()
        );
    }
    let clang_path_string = format!("{}/{}", wasi_sdk_path_string, "bin/clang");
    let clang_path = std::path::Path::new(&clang_path_string);
    if !clang_path.exists() {
        panic!(
            "WASI_SDK_PATH is set to a path that does not contain clang: {}",
            clang_path.display()
        );
    }
    vec![
        ("WASI_SDK_PATH", wasi_sdk_path_string),
        ("CC_wasm32_wasi", clang_path_string),
    ]
}

fn check_cargo_component_installed() {
    let (_, output) = run(["cargo", "component", "--version"], ".", std::iter::empty());
    if !output.status.success() {
        panic!("cargo-component is not installed. Please install it by running `cargo install cargo-component`");
    }
}

enum Target {
    Wasm32UnknownUnknown,
    Wasm32Wasi,
}

fn cargo_component_build<'a>(
    dir: &str,
    target: Target,
    env: impl Iterator<Item = (&'a str, &'a str)>,
) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let (cmd, output) = run(
        [
            "cargo",
            "component",
            "build",
            "--target",
            match target {
                Target::Wasm32UnknownUnknown => "wasm32-unknown-unknown",
                Target::Wasm32Wasi => "wasm32-wasi",
            },
            "--release",
            "--target-dir",
            out_dir.to_str().unwrap(),
        ],
        dir,
        env,
    );
    if !output.status.success() {
        println!("{}", std::str::from_utf8(&output.stderr).unwrap());
        println!("{}", std::str::from_utf8(&output.stdout).unwrap());
        panic!("while running the build script, the command '{cmd}' failed to run in '{dir}'")
    }
    println!("cargo:rerun-if-changed={dir}/Cargo.toml");
    println!("cargo:rerun-if-changed={dir}/src");
}

fn run<'a, 'b>(
    args: impl IntoIterator<Item = &'a str> + 'a,
    dir: &str,
    env: impl Iterator<Item = (&'b str, &'b str)>,
) -> (String, process::Output) {
    let mut cmd = process::Command::new(get_os_process());
    cmd.stdout(process::Stdio::inherit());
    cmd.stderr(process::Stdio::piped());
    cmd.current_dir(dir);

    cmd.arg("-c");
    cmd.envs(env);
    let c = args
        .into_iter()
        .map(Into::into)
        .collect::<Vec<&str>>()
        .join(" ");
    cmd.arg(&c);

    (c, cmd.output().unwrap())
}

fn get_os_process() -> String {
    if cfg!(target_os = "windows") {
        String::from("powershell.exe")
    } else {
        String::from("bash")
    }
}
