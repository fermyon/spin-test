use std::{collections::BTreeMap, fs};
use wasi_virt::{FsEntry, WasiVirt};

fn main() {
    let mut virt = WasiVirt::new();

    // allow all subsystems initially
    virt.allow_all();

    // ignore stdio
    virt.stdio().ignore();

    virt.http(false);
    virt.sockets(false);

    virt.env()
        .overrides(&[("SOME", "ENV"), ("VAR", "OVERRIDES")]);

    virt.fs()
        // deny arbitrary host preopens
        .deny_host_preopens();
    // mount and virtualize a local directory recursively
    virt.fs()
        // create a virtual directory containing some virtual files
        .preopen(
            "/another-dir".into(),
            FsEntry::Dir(BTreeMap::from([
                // create a virtual file from the given UTF8 source
                ("file.txt".into(), FsEntry::Source("Hello world".into())),
            ])),
        );

    let virt_component_bytes = virt.finish().unwrap();
    fs::write("virt.component.wasm", &virt_component_bytes.adapter).unwrap();
}
