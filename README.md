# `spin-test`

`spin-test` is a plugin for Spin that runs tests written in WebAssembly against a Spin application where all Spin and WASI APIs are configurable and assertable mocks.

## Usage

`spin-test` can be used as a stand alone binary or as a plugin for Spin.

### Install `spin test` (plugin)

To install the `spin test` plugin canary release, run the following:

```
spin plugin install -u https://github.com/fermyon/spin-test/releases/download/canary/spin-test.json
```

This will install the plugin which can be invoked with `spin test`.

In the future, stable, non-canary releases will also be made available.

#### Installing a Locally Built Version

To install a version of `spin-test` plugin that has been locally built using `cargo build --release`, use the following command:

```bash
spin pluginify -i
```

Note: the [`pluginify`](https://github.com/fermyon/spin-plugins/blob/main/manifests/pluginify/pluginify.json) plugin is a pre-requisite.

### Install `spin-test` (stand alone)

Alternatively, to install `spin-test` as a stand alone binary, run `cargo build --release` from this directory and ensure that the resulting binary is located on your path.

If you'd rather not build from source, you can find a pre-built binary inside the plugin tarballs included in [any release](https://github.com/fermyon/spin-test/releases). In the latest stable release or in the canary release, find the tarball asset corresponding to the appropriate machine architecture, download the tarball, unarchive it, and retrieve the `test` binary inside it. Place this binary somewhere on your path as `spin-test` and invoke by running `spin-test`.

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