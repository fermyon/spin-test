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
