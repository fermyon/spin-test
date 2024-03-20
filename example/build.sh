set -ex

# Build the spin app
pushd guest
spin build
popd

# Copy the spin app module into the right place and componentize
cp ~/.cargo_target/wasm32-wasi/release/guest.wasm deps/example
wasm-tools component new --adapt wasi_snapshot_preview1=preview1-adapter.wasm deps/example/guest.wasm -o deps/example/guest.wasm

# Encode the composition
wac encode composition.wac -o composition.wasm
