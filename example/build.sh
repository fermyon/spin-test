set -ex

# Build the spin app
pushd guest
spin build
popd

TARGET="${CARGO_TARGET_DIR:-../target}"
# Copy the spin app module into the right place and componentize
cp $TARGET/wasm32-wasi/release/guest.wasm deps/example
wasm-tools component new --adapt wasi_snapshot_preview1=preview1-adapter.wasm deps/example/guest.wasm -o deps/example/guest.wasm
wasm-tools strip deps/example/guest.wasm -o deps/example/guest.wasm

# Encode the composition
wac encode composition.wac -o composition.wasm
wasm-tools strip composition.wasm -o composition.wasm
