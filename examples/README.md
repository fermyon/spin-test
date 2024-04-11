# `spin-test` Examples

The examples folder contains multiple examples of `spin-test` compliant tests along with example applications (found in the `apps` directory) that the tests can be run against.

## Running

Running a test against a Spin application requires the following steps:

* Build the test wasm binary. Each test directory contains a README.md file with instructions on how to build the test since this is language dependent.
* Build the Spin application. Change into the app directory where the app you want to test lives, and run `spin build`.
* Run `spin-test`. From inside the directory for the app you're testing, run `spin-test $PATH_TO_TEST_BINARY`
