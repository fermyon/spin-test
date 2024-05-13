use std::{env, process};

fn main() {
    check_cargo_component_installed();
    cargo_component_build("crates/router");
    cargo_component_build("crates/spin-test-virt");
}

fn check_cargo_component_installed() {
    let (_, output) = run(["cargo", "component", "--version"], ".");
    if !output.status.success() {
        panic!("cargo-component is not installed. Please install it by running `cargo install cargo-component`");
    }
}

fn cargo_component_build(dir: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let (cmd, output) = run(
        [
            "cargo",
            "component",
            "build",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--target-dir",
            out_dir.to_str().unwrap(),
        ],
        dir,
    );
    if !output.status.success() {
        println!("{}", std::str::from_utf8(&output.stderr).unwrap());
        println!("{}", std::str::from_utf8(&output.stdout).unwrap());
        panic!("while running the build script, the command '{cmd}' failed to run in '{dir}'")
    }
    println!("cargo:rerun-if-changed={dir}/Cargo.toml");
    println!("cargo:rerun-if-changed={dir}/src");
}

fn run<'a>(args: impl IntoIterator<Item = &'a str> + 'a, dir: &str) -> (String, process::Output) {
    let mut cmd = process::Command::new(get_os_process());
    cmd.stdout(process::Stdio::inherit());
    cmd.stderr(process::Stdio::piped());
    cmd.current_dir(dir);

    cmd.arg("-c");
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
