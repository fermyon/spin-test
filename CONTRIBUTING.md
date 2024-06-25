# Contributing

## Building from source

To build `spin-test` from source, you'll need to [download the `WASI_SDK`](https://github.com/WebAssembly/wasi-sdk/releases/) (needed for the C compiler used to compile some C dependencies). Once you have the SDK on your machine somewhere, point to it via the `WASI_SDK_PATH` environment variable.

You can then run `cargo build --release` to build `spin-test`.