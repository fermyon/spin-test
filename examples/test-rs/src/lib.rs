use bindings::{
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

mod bindings;

struct Component;

impl bindings::Guest for Component {
    fn run() {
        // Configure the test
        let user = r#"{"id":123,"name":"Ryan"}"#;

        let key_value_config = key_value::Store::open("cache").unwrap();
        // Set state of the key-value store
        key_value_config.set("123", user.as_bytes()).unwrap();
        key_value_calls::reset_calls();

        let request = OutgoingRequest::new(Headers::new());
        request.set_path_with_query(Some("/?user_id=123")).unwrap();
        let request = new_request(request);
        let (response_out, response_receiver) = new_response();
        incoming_handler::handle(request, response_out);
        let response = response_receiver.get().unwrap();
        assert_eq!(response.status(), 200);

        let mut body = String::new();
        read_body(response.consume().unwrap(), |buffer| {
            body.push_str(&String::from_utf8(buffer).unwrap())
        })
        .unwrap();
        assert_eq!(body, user);

        let calls = key_value_calls::calls()
            .into_iter()
            .find_map(|(key, value)| (key == "cache").then_some(value))
            .unwrap_or_default();
        assert_eq!(calls, vec![key_value_calls::Call::Get("123".to_owned())]);
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

bindings::export!(Component with_types_in bindings);
