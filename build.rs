use std::{env, process};

fn main() {
    cargo_component_build("crates/router");
    cargo_component_build("crates/spin-test-virt");
}

fn cargo_component_build(dir: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    run(
        [
            "cargo",
            "component",
            "build",
            "--release",
            "--target-dir",
            out_dir.to_str().unwrap(),
        ],
        dir,
    );
    println!("cargo:rerun-if-changed={dir}/Cargo.toml");
    println!("cargo:rerun-if-changed={dir}/src");
}

fn run<'a>(args: impl IntoIterator<Item = &'a str> + 'a, dir: &str) {
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

    let output = cmd.output().unwrap();
    let exit = output.status;
    if !exit.success() {
        println!("{}", std::str::from_utf8(&output.stderr).unwrap());
        println!("{}", std::str::from_utf8(&output.stdout).unwrap());
        panic!("while running the build script, the command '{c}' failed to run in '{dir}'")
    }
}

fn get_os_process() -> String {
    if cfg!(target_os = "windows") {
        String::from("powershell.exe")
    } else {
        String::from("bash")
    }
}
