use std::{
    env,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process,
};

fn main() {
    check_cargo_component_installed();
    cargo_component_build("crates/router");
    cargo_component_build("crates/spin-test-virt");
    copy_wit_to_out_dir();
}

/// Make the wit files available in the out directory
fn copy_wit_to_out_dir() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("world.wit");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out)
        .unwrap();
    pack_dir_all("host-wit", &mut file).unwrap();
    println!("cargo:rerun-if-changed=host-wit");
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

/// Copy the contents of a `src` directory to a single `dst` file
///
/// Each file in `src` is written to `dst` in the following format:
/// * Path length (u16, big-endian)
/// * Path (utf-8)
/// * File length (u64, big-endian)
/// * File contents
fn pack_dir_all(src: impl AsRef<Path>, dst: &mut std::fs::File) -> std::io::Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            pack_dir_all(entry.path(), dst)?;
        } else {
            let path = entry.path().display().to_string();
            dst.write_all(&(path.len() as u16).to_be_bytes())?;
            write!(dst, "{path}")?;
            dst.write_all(&entry.metadata()?.len().to_be_bytes())?;
            let mut reader =
                std::io::BufReader::with_capacity(1024 * 128, std::fs::File::open(entry.path())?);
            loop {
                let buffer = reader.fill_buf()?;
                let length = buffer.len();
                if length == 0 {
                    break;
                } else {
                    dst.write_all(buffer)?;
                }
                reader.consume(length);
            }
        }
    }
    Ok(())
}
