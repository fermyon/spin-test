use std::fs;
use wasi_virt::WasiVirt;

fn main() {
    // Create a new WasiVirt instance
    // By default this will not virtualize anything.
    let mut virt = WasiVirt::new();

    /// Add some environment variables to the virtualized environment
    virt.env().deny_all().overrides(&[("x-spin-test", "true")]);

    let virt_component_bytes = virt.finish().unwrap();
    fs::write("virt.component.wasm", &virt_component_bytes.adapter).unwrap();
}
