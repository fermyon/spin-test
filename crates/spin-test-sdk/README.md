# `spin-test` Rust SDK

A library for writing `spin-test` tests in Rust.

## Usage

The `spin-test` Rust SDK allows you to annotate functions as `spin-test` tests that will automatically be exposed to `spin-test` as tests to be run: 

```rust
use spin_test_sdk::{bindings::wasi::http, spin_test};

#[spin_test]
fn my_test() {
    // Make a request to the Spin application
    let request = http::types::OutgoingRequest::new(http::types::Headers::new());
    request.set_path_with_query(Some("/?user_id=123")).unwrap();
    let response = spin_test_sdk::perform_request(request);

    // Assert response status
    assert_eq!(response.status(), 200);
}
```

The `spin-test` Rust SDK gives access to the `fermyon:spin/test` world through the `bindings` module. You can use the various types in that module to customize your test scenario, make requests against the Spin application, and make assertions about the state of the world before and after the request has been made.

As an example, in the following code, we are seeding the `key-value` store our Spin app has access to with some values to test that our app can handle when the key-value store is in such a state:

```rust
    // Open the store
    let key_value = key_value::Store::open("default");
    // Set state of the key-value store
    key_value.set("123", "abc");
    // Now make our request to the Spin app...
```
