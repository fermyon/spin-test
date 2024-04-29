use std::cell::RefCell;

use crate::Component;

use crate::bindings::exports::fermyon::spin_wasi_virt::http_helper as exports;

use super::http::{Fields, IncomingBody, IncomingRequest, OutgoingRequest, ResponseOutparam};

impl exports::Guest for Component {
    type ResponseReceiver = ResponseReceiver;

    fn new_request(
        request: exports::OutgoingRequest,
        incoming_body: Option<exports::IncomingBody>,
    ) -> exports::IncomingRequest {
        let request: OutgoingRequest = request.into_inner();
        let method = request.method.into_inner();
        let scheme = request.scheme.into_inner();
        let authority = request.authority.into_inner();
        let path_with_query = request.path_with_query.into_inner();
        let headers: Fields = request.headers;
        let body: Option<IncomingBody> = incoming_body.map(|b| b.into_inner());
        exports::IncomingRequest::new(IncomingRequest {
            method,
            scheme,
            authority,
            path_with_query,
            headers,
            body: RefCell::new(Some(body.unwrap_or_else(|| IncomingBody))),
        })
    }

    fn new_response() -> (exports::ResponseOutparam, exports::ResponseReceiver) {
        (
            exports::ResponseOutparam::new(ResponseOutparam),
            exports::ResponseReceiver::new(ResponseReceiver),
        )
    }
}

pub struct ResponseReceiver;

impl exports::GuestResponseReceiver for ResponseReceiver {
    fn get(&self) -> Option<exports::IncomingResponse> {
        todo!()
    }
}
