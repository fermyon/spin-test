# `spin-test`

`spin-test` is a plugin for Spin that runs tests written in WebAssembly against a Spin application where all Spin and WASI APIs are configurable and assertable mocks.

## Usage

### Create a Spin App

`spin-test` runs tests against a Spin application. As such, you'll need a Spin app to test against. You can find more information on creating Spin applications [here](https://developer.fermyon.com/spin/v2/quickstart).

### Create a `spin-test` test

Next you'll need a `spin-test` test compiled to a WebAssembly component.

There is currently first class support for Rust through the [`spin-test` Rust SDK](./crates/spin-test-sdk/), but any language with support for writing WebAssembly components can be used as long as the `fermyon:spin-test/test` world is targeted. You can find the definition of this world in [here](./host-wit/world.wit).

You can see examples of tests written in a variety of languages in [the examples](./examples/).

### Configure `spin-test`

Next, we'll need to tell `spin-test` where our test lives and how to build it. We do this from inside the `spin.toml` manifest. Let's imagine our app has a component named "my-component" that we want to test. In the `spin.toml` manifest we can add the following configuration:

```toml
[component.my-component.tool.spin-test]
# A relative path to where the built test component binary will live.
source = "target/wasm32-wasi/release/test.wasm"
# A command for building the target component.
build = "cargo component build --release"
# The directory where the `build` command should be run.
dir = "../../test-rs"
```

### Run `spin-test`

Finally, we're ready for our test to be run. We can do this simply by invoking `spin-test` from the directory where our Spin application lives:

```bash
spin-test
```

## Examples

See the [`examples`](./examples/) directory for a few examples of `spin-test` tests that test the apps in the [`apps`](./examples/apps/) directory.