pub use spin_test_sdk_macro::spin_test;

pub use wit::{
    fermyon::{
        spin::key_value,
        spin_test::http_helper::{new_request, new_response},
        spin_test_virt::key_value_calls,
    },
    wasi::{
        http::types::{Headers, InputStream, OutgoingRequest},
        http::{incoming_handler, types::IncomingBody},
        io::streams::{self, StreamError},
    },
};

pub mod wit {
    wit_bindgen::generate!({
        world: "test-imports",
        path: "../../host-wit",
    });
}

#[doc(hidden)]
pub use wit_bindgen;

pub mod http {
    use super::{streams, IncomingBody, InputStream, StreamError};

    const READ_SIZE: u64 = 16 * 1024;
    pub fn read_body(
        body: IncomingBody,
        mut callback: impl FnMut(Vec<u8>),
    ) -> Result<(), streams::Error> {
        struct Incoming(Option<(InputStream, IncomingBody)>);

        impl Drop for Incoming {
            fn drop(&mut self) {
                if let Some((stream, body)) = self.0.take() {
                    drop(stream);
                    IncomingBody::finish(body);
                }
            }
        }

        let stream = body.stream().expect("response body should be readable");
        let pair = Incoming(Some((stream, body)));

        loop {
            if let Some((stream, _)) = &pair.0 {
                match stream.blocking_read(READ_SIZE) {
                    Ok(buffer) => callback(buffer),
                    Err(StreamError::Closed) => return Ok(()),
                    Err(StreamError::LastOperationFailed(error)) => return Err(error),
                }
            }
        }
    }
}

impl PartialEq for key_value_calls::Call {
    fn eq(&self, other: &Self) -> bool {
        use key_value_calls::Call::*;
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
