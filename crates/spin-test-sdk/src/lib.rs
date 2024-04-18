pub use spin_test_sdk_macro::spin_test;

mod type_extensions;

/// Raw bindings to everything available to the test.
pub mod bindings {
    wit_bindgen::generate!({
        world: "test-imports",
        path: "../../host-wit",
    });
}

use bindings::fermyon::spin_test;
use bindings::wasi::http;

/// Configure and assert the behavior of the key-value store.
pub mod key_value {
    use std::ops::Deref;

    use super::bindings::fermyon::spin::key_value;
    use super::bindings::fermyon::spin_test_virt::key_value_calls;

    #[doc(inline)]
    pub use key_value_calls::Call;

    /// A wrapper around the key-value store.
    pub struct Store {
        name: String,
        inner: key_value::Store,
    }

    impl Store {
        /// Open a key-value store.
        pub fn open(name: &str) -> Result<Self, key_value::Error> {
            Ok(Self {
                inner: key_value::Store::open(name)?,
                name: name.to_owned(),
            })
        }

        pub fn calls(&self) -> Vec<key_value_calls::Call> {
            key_value_calls::calls()
                .into_iter()
                .find_map(|(key, value)| (key == self.name).then_some(value))
                .unwrap_or_default()
        }

        /// Reset the call history.
        pub fn reset_calls(&self) {
            key_value_calls::reset_calls();
        }
    }

    impl Deref for Store {
        type Target = key_value::Store;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl Drop for Store {
        fn drop(&mut self) {
            self.reset_calls();
        }
    }
}

/// Make a request to the Spin app and return the response.
pub fn perform_request(request: http::types::OutgoingRequest) -> http::types::IncomingResponse {
    let request = spin_test::http_helper::new_request(request);
    let (response_out, response_receiver) = spin_test::http_helper::new_response();
    http::incoming_handler::handle(request, response_out);
    response_receiver.get().unwrap()
}

#[doc(hidden)]
/// This module is used by the `spin_test` macro for hooking into the wit_bindgen runtime.
pub use wit_bindgen;
