use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use crate::bindings::exports::fermyon::spin_wasi_virt::http_helper as exports;
use crate::Component;

use super::{
    http::{
        self, Fields, IncomingBody, IncomingRequest, IncomingResponse, OutgoingRequest,
        OutgoingResponse, ResponseOutparam,
    },
    io,
};

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
            body: body.unwrap_or_else(IncomingBody::empty).into(),
        })
    }

    fn new_response() -> (exports::ResponseOutparam, exports::ResponseReceiver) {
        let response = Arc::new(Mutex::new(None));
        (
            exports::ResponseOutparam::new(ResponseOutparam(response.clone())),
            exports::ResponseReceiver::new(ResponseReceiver(response)),
        )
    }
}

pub struct ResponseReceiver(
    Arc<
        Mutex<
            Option<
                Result<
                    super::http::exports::types::OutgoingResponse,
                    super::http::exports::types::ErrorCode,
                >,
            >,
        >,
    >,
);

impl exports::GuestResponseReceiver for ResponseReceiver {
    fn get(&self) -> Option<exports::IncomingResponse> {
        let response = match &*self.0.lock().unwrap() {
            Some(Ok(r)) => {
                let outgoing = r.get::<OutgoingResponse>();
                Some(IncomingResponse {
                    status: outgoing.status_code.get(),
                    headers: outgoing.headers.clone(),
                    body: outgoing.body.unconsume().map(Into::into),
                })
            }
            Some(Err(e)) => {
                use http::exports::types::ErrorCode;
                let (status, msg) = match &e {
                    ErrorCode::InternalError(Some(msg)) => (500, msg.clone()),
                    ErrorCode::InternalError(None) => {
                        (500, "an internal error occurred".to_owned())
                    }
                    ErrorCode::DestinationNotFound => (
                        404,
                        "no route found in spin.toml manifest for request path".to_owned(),
                    ),
                    _ => (500, e.to_string()),
                };
                Some(IncomingResponse {
                    status,
                    headers: Fields::default(),
                    body: IncomingBody::new(msg.into()).into(),
                })
            }
            None => None,
        };
        response.map(exports::IncomingResponse::new)
    }
}
