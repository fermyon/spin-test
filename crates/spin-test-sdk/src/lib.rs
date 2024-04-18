pub use spin_test_sdk_macro::spin_test;

mod bindings {
    wit_bindgen::generate!({
        world: "test-imports",
        path: "../../host-wit",
    });
}

#[doc(hidden)]
pub use wit_bindgen;

pub mod wasi {
    use super::bindings;
    pub use bindings::wasi::{clocks, io};
    pub mod http {
        pub use super::bindings::wasi::http::*;
        use super::io::streams;

        impl types::IncomingBody {
            /// Read the body of the incoming request calling the callback on each chunk.
            pub fn read(self, mut callback: impl FnMut(Vec<u8>)) -> Result<(), streams::Error> {
                struct Incoming(Option<(streams::InputStream, types::IncomingBody)>);

                impl Drop for Incoming {
                    fn drop(&mut self) {
                        if let Some((stream, body)) = self.0.take() {
                            drop(stream);
                            types::IncomingBody::finish(body);
                        }
                    }
                }

                let stream = self.stream().expect("response body should be readable");
                let pair = Incoming(Some((stream, self)));

                loop {
                    if let Some((stream, _)) = &pair.0 {
                        const READ_SIZE: u64 = 16 * 1024;
                        match stream.blocking_read(READ_SIZE) {
                            Ok(buffer) => callback(buffer),
                            Err(streams::StreamError::Closed) => return Ok(()),
                            Err(streams::StreamError::LastOperationFailed(error)) => {
                                return Err(error)
                            }
                        }
                    }
                }
            }

            pub fn read_to_string(self) -> Result<String, streams::Error> {
                let mut result = String::new();
                self.read(|buffer| result.push_str(&String::from_utf8(buffer).unwrap()))?;
                Ok(result)
            }
        }
    }
}

pub mod fermyon {
    pub use super::bindings::fermyon::{spin, spin_test_virt};
    pub mod spin_test {
        pub use super::super::bindings::fermyon::spin_test::*;

        /// Make a request to the HTTP handler and return the response.
        pub fn perform_request(
            request: crate::wasi::http::types::OutgoingRequest,
        ) -> crate::wasi::http::types::IncomingResponse {
            let request = http_helper::new_request(request);
            let (response_out, response_receiver) = http_helper::new_response();
            crate::wasi::http::incoming_handler::handle(request, response_out);
            response_receiver.get().unwrap()
        }
    }
}

impl PartialEq for fermyon::spin_test_virt::key_value_calls::Call {
    fn eq(&self, other: &Self) -> bool {
        use fermyon::spin_test_virt::key_value_calls::Call::*;
        match (self, other) {
            (Get(a), Get(b)) => a == b,
            (Set(a), Set(b)) => a == b,
            (Delete(a), Delete(b)) => a == b,
            (Exists(a), Exists(b)) => a == b,
            (GetKeys, GetKeys) => true,
            _ => false,
        }
    }
}
