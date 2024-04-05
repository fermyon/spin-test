use std::sync::Arc;

const SPIN_TEST_VIRT: &[u8] = include_bytes!("../../../example/deps/fermyon/spin-test-virt.wasm");
const WASI_VIRT: &[u8] = include_bytes!("../../../example/deps/wasi/virt.wasm");
const ROUTER: &[u8] = include_bytes!("../../../example/deps/fermyon/router.wasm");

fn main() {
    env_logger::init();
    let app = std::fs::read(
        std::env::args()
            .nth(1)
            .expect("first argument should be the path to the app wasm"),
    )
    .unwrap();
    let app = wac_graph::types::Package::from_bytes("app", None, Arc::new(app)).unwrap();
    let virt = wac_graph::types::Package::from_bytes(
        "spin-test-virt",
        None,
        Arc::new(SPIN_TEST_VIRT.to_owned()),
    )
    .unwrap();
    let wasi_virt =
        wac_graph::types::Package::from_bytes("wasi-virt", None, Arc::new(WASI_VIRT.to_owned()))
            .unwrap();
    let router =
        wac_graph::types::Package::from_bytes("router", None, Arc::new(ROUTER.to_owned())).unwrap();

    let mut graph = wac_graph::CompositionGraph::new();
    let app = graph.register_package(app).unwrap();
    let virt = graph.register_package(virt).unwrap();
    // let wasi_virt = graph.register_package(wasi_virt).unwrap();
    // let router = graph.register_package(router).unwrap();

    let virt = graph.instantiate(virt).unwrap();
    let app = graph.instantiate(app).unwrap();
    // let wasi_virt = graph.instantiate(wasi_virt).unwrap();
    // let router = graph.instantiate(router).unwrap();

    // let key_value = graph
    //     .alias_instance_export(virt, "fermyon:spin/key-value@2.0.0")
    //     .unwrap();
    // graph
    //     .connect_argument(key_value, app, "fermyon:spin/key-value@2.0.0")
    //     .unwrap();

    let outgoing_handler = graph
        .alias_instance_export(virt, "wasi:http/outgoing-handler@0.2.0")
        .unwrap();
    graph
        .connect_argument(outgoing_handler, app, "wasi:http/outgoing-handler@0.2.0")
        .unwrap();

    // let env = graph
    //     .alias_instance_export(wasi_virt, "wasi:cli/environment@0.2.0")
    //     .unwrap();
    // graph
    //     .connect_argument(env, app, "wasi:cli/environment@0.2.0")
    //     .unwrap();

    // let incoming_handler = graph
    //     .alias_instance_export(app, "wasi:http/incoming-handler@0.2.0")
    //     .unwrap();
    // graph
    //     .connect_argument(incoming_handler, router, "wasi:http/incoming-handler@0.2.0")
    //     .unwrap();

    // let set_component_id = graph
    //     .alias_instance_export(virt, "set-component-id")
    //     .unwrap();
    // graph
    //     .connect_argument(set_component_id, router, "set-component-id")
    //     .unwrap();

    // let incoming_handler_export = graph
    //     .alias_instance_export(router, "wasi:http/incoming-handler@0.2.0")
    //     .unwrap();
    // graph
    //     .export(incoming_handler_export, "wasi:http/incoming-handler@0.2.0")
    //     .unwrap();

    let encoded = graph
        .encode(wac_graph::EncodeOptions {
            validate: true,
            ..Default::default()
        })
        .unwrap();
    std::fs::write("composed.wasm", encoded).unwrap();
}
