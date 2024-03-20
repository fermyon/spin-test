# `spin-virt` Example

The example is composed of several parts:
* `guest`: a plain ol' Spin application that uses various parts of the Spin API
* `composition.wac` and `deps`: parts used to run the `wac` tool for composing the `guest`
with `spin-virt`.
* `host`: a host that knows how to run the composed component.

## Running

First, build the guest and compose it with `spin-virt`:

```bash
./build.sh
```

Then run the host:

```bash
./run.sh
```