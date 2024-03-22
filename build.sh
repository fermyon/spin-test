set -ex

TARGET="${CARGO_TARGET_DIR:-target}"

cargo component b --target=wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/spin_test_virt.wasm example/deps/fermyon/spin-test-virt.wasm
