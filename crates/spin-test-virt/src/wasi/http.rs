use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    collections::HashMap,
    sync::Mutex,
};

pub use crate::bindings::exports::wasi::http as exports;
pub use crate::bindings::wasi::http as imports;

use crate::Component;

use super::io;

impl exports::types::Guest for Component {
    type Fields = Fields;

    type IncomingRequest = IncomingRequest;

    type OutgoingRequest = OutgoingRequest;

    type RequestOptions = RequestOptions;

    type ResponseOutparam = ResponseOutparam;

    type IncomingResponse = IncomingResponse;

    type IncomingBody = IncomingBody;

    type FutureTrailers = FutureTrailers;

    type OutgoingResponse = OutgoingResponse;

    type OutgoingBody = OutgoingBody;

    type FutureIncomingResponse = FutureIncomingResponse;

    fn http_error_code(
        err: io::exports::error::ErrorBorrow<'_>,
    ) -> Option<exports::types::ErrorCode> {
        todo!()
    }
}

pub struct FutureIncomingResponse;

impl exports::types::GuestFutureIncomingResponse for FutureIncomingResponse {
    fn subscribe(&self) -> io::exports::poll::Pollable {
        todo!()
    }

    fn get(
        &self,
    ) -> Option<Result<Result<exports::types::IncomingResponse, exports::types::ErrorCode>, ()>>
    {
        todo!()
    }
}

pub struct OutgoingBody;

impl exports::types::GuestOutgoingBody for OutgoingBody {
    fn write(&self) -> Result<io::exports::streams::OutputStream, ()> {
        todo!()
    }

    fn finish(
        this: exports::types::OutgoingBody,
        trailers: Option<exports::types::Trailers>,
    ) -> Result<(), exports::types::ErrorCode> {
        todo!()
    }
}

pub struct OutgoingResponse;

impl exports::types::GuestOutgoingResponse for OutgoingResponse {
    fn new(headers: exports::types::Headers) -> Self {
        todo!()
    }

    fn status_code(&self) -> exports::types::StatusCode {
        todo!()
    }

    fn set_status_code(&self, status_code: exports::types::StatusCode) -> Result<(), ()> {
        todo!()
    }

    fn headers(&self) -> exports::types::Headers {
        todo!()
    }

    fn body(&self) -> Result<exports::types::OutgoingBody, ()> {
        todo!()
    }
}

pub struct FutureTrailers;

impl exports::types::GuestFutureTrailers for FutureTrailers {
    fn subscribe(&self) -> io::exports::poll::Pollable {
        todo!()
    }

    fn get(
        &self,
    ) -> Option<Result<Result<Option<exports::types::Trailers>, exports::types::ErrorCode>, ()>>
    {
        todo!()
    }
}

pub struct IncomingBody;

impl exports::types::GuestIncomingBody for IncomingBody {
    fn stream(&self) -> Result<io::exports::streams::InputStream, ()> {
        todo!()
    }

    fn finish(this: exports::types::IncomingBody) -> exports::types::FutureTrailers {
        todo!()
    }
}

pub struct IncomingResponse {
    body: RefCell<Option<exports::types::IncomingBody>>,
}

impl exports::types::GuestIncomingResponse for IncomingResponse {
    fn status(&self) -> exports::types::StatusCode {
        todo!()
    }

    fn headers(&self) -> exports::types::Headers {
        todo!()
    }

    fn consume(&self) -> Result<exports::types::IncomingBody, ()> {
        self.body.borrow_mut().take().ok_or(())
    }
}

pub struct ResponseOutparam;

impl exports::types::GuestResponseOutparam for ResponseOutparam {
    fn set(
        param: exports::types::ResponseOutparam,
        response: Result<exports::types::OutgoingResponse, exports::types::ErrorCode>,
    ) {
        todo!()
    }
}

pub struct RequestOptions;

impl exports::types::GuestRequestOptions for RequestOptions {
    fn new() -> Self {
        todo!()
    }

    fn connect_timeout(&self) -> Option<exports::types::Duration> {
        todo!()
    }

    fn set_connect_timeout(&self, duration: Option<exports::types::Duration>) -> Result<(), ()> {
        todo!()
    }

    fn first_byte_timeout(&self) -> Option<exports::types::Duration> {
        todo!()
    }

    fn between_bytes_timeout(&self) -> Option<exports::types::Duration> {
        todo!()
    }

    fn set_between_bytes_timeout(
        &self,
        duration: Option<exports::types::Duration>,
    ) -> Result<(), ()> {
        todo!()
    }

    fn set_first_byte_timeout(&self, duration: Option<exports::types::Duration>) -> Result<(), ()> {
        todo!()
    }
}

pub struct OutgoingRequest {
    pub method: RefCell<exports::types::Method>,
    pub scheme: RefCell<Option<exports::types::Scheme>>,
    pub authority: RefCell<Option<String>>,
    pub path_with_query: RefCell<Option<String>>,
    pub headers: Fields,
}

impl exports::types::GuestOutgoingRequest for OutgoingRequest {
    fn new(headers: exports::types::Headers) -> Self {
        let headers = headers.into_inner();
        Self {
            method: RefCell::new(exports::types::Method::Get),
            scheme: Default::default(),
            authority: Default::default(),
            path_with_query: Default::default(),
            headers,
        }
    }

    fn body(&self) -> Result<exports::types::OutgoingBody, ()> {
        todo!()
    }

    fn method(&self) -> exports::types::Method {
        self.method.borrow().clone()
    }

    fn set_method(&self, method: exports::types::Method) -> Result<(), ()> {
        // TODO: check for syntactic correctness of `method.other`
        *self.method.borrow_mut() = method;
        Ok(())
    }

    fn path_with_query(&self) -> Option<String> {
        self.path_with_query.borrow().clone()
    }

    fn set_path_with_query(&self, path_with_query: Option<String>) -> Result<(), ()> {
        // TODO: check for syntactic correctness of `path_with_query`
        *self.path_with_query.borrow_mut() = path_with_query;
        Ok(())
    }

    fn scheme(&self) -> Option<exports::types::Scheme> {
        self.scheme.borrow().clone()
    }

    fn set_scheme(&self, scheme: Option<exports::types::Scheme>) -> Result<(), ()> {
        // TODO: check for syntactic correctness of `scheme`
        *self.scheme.borrow_mut() = scheme;
        Ok(())
    }

    fn authority(&self) -> Option<String> {
        self.authority.borrow().clone()
    }

    fn set_authority(&self, authority: Option<String>) -> Result<(), ()> {
        // TODO: check for syntactic correctness of `authority`
        *self.authority.borrow_mut() = authority;
        Ok(())
    }

    fn headers(&self) -> exports::types::Headers {
        exports::types::Headers::new(self.headers.clone())
    }
}

pub struct IncomingRequest {
    pub method: exports::types::Method,
    pub scheme: Option<exports::types::Scheme>,
    pub authority: Option<String>,
    pub path_with_query: Option<String>,
    pub headers: Fields,
    pub body: RefCell<Option<IncomingBody>>,
}

impl exports::types::GuestIncomingRequest for IncomingRequest {
    fn method(&self) -> exports::types::Method {
        self.method.clone()
    }

    fn path_with_query(&self) -> Option<String> {
        self.path_with_query.clone()
    }

    fn scheme(&self) -> Option<exports::types::Scheme> {
        self.scheme.clone()
    }

    fn authority(&self) -> Option<String> {
        self.authority.clone()
    }

    fn headers(&self) -> exports::types::Headers {
        exports::types::Headers::new(self.headers.clone())
    }

    fn consume(&self) -> Result<exports::types::IncomingBody, ()> {
        let body = self.body.borrow_mut().take().ok_or(())?;
        Ok(exports::types::IncomingBody::new(body))
    }
}

#[derive(Debug, Default, Clone)]
pub struct Fields {
    fields: RefCell<HashMap<exports::types::FieldKey, Vec<exports::types::FieldValue>>>,
}

impl exports::types::GuestFields for Fields {
    fn new() -> Self {
        Self::default()
    }

    fn from_list(
        entries: Vec<(exports::types::FieldKey, exports::types::FieldValue)>,
    ) -> Result<exports::types::Fields, exports::types::HeaderError> {
        todo!()
    }

    fn get(&self, name: exports::types::FieldKey) -> Vec<exports::types::FieldValue> {
        self.fields
            .borrow()
            .get(&name)
            .map(Clone::clone)
            .unwrap_or_default()
    }

    fn has(&self, name: exports::types::FieldKey) -> bool {
        todo!()
    }

    fn set(
        &self,
        name: exports::types::FieldKey,
        value: Vec<exports::types::FieldValue>,
    ) -> Result<(), exports::types::HeaderError> {
        todo!()
    }

    fn delete(&self, name: exports::types::FieldKey) -> Result<(), exports::types::HeaderError> {
        todo!()
    }

    fn append(
        &self,
        name: exports::types::FieldKey,
        value: exports::types::FieldValue,
    ) -> Result<(), exports::types::HeaderError> {
        // TODO: check for mutability rules
        self.fields
            .borrow_mut()
            .entry(name)
            .or_default()
            .push(value);
        Ok(())
    }

    fn entries(&self) -> Vec<(exports::types::FieldKey, exports::types::FieldValue)> {
        self.fields
            .borrow()
            .iter()
            .flat_map(|(k, v)| v.clone().into_iter().map(move |v| (k.clone(), v)))
            .collect()
    }

    fn clone(&self) -> exports::types::Fields {
        todo!()
    }
}

pub static RESPONSES: std::sync::OnceLock<
    Mutex<HashMap<String, exports::types::OutgoingResponse>>,
> = std::sync::OnceLock::new();

impl exports::outgoing_handler::Guest for Component {
    fn handle(
        request: exports::outgoing_handler::OutgoingRequest,
        _options: Option<exports::outgoing_handler::RequestOptions>,
    ) -> Result<
        exports::outgoing_handler::FutureIncomingResponse,
        exports::outgoing_handler::ErrorCode,
    > {
        let request: OutgoingRequest = request.into_inner();
        let url = format!(
            "{scheme}://{authority}{path_and_query}",
            scheme = match &*request.scheme.borrow() {
                Some(exports::types::Scheme::Http) => "http",
                Some(exports::types::Scheme::Https) | None => "https",
                Some(exports::types::Scheme::Other(ref s)) => s,
            },
            authority = request.authority.borrow().as_ref().expect("TODO: handle"),
            path_and_query = request
                .path_with_query
                .borrow()
                .as_ref()
                .map(Clone::clone)
                .filter(|p| p != "/")
                .unwrap_or_default()
        );
        let url_allowed = crate::manifest::AppManifest::allows_url(&url, "https").map_err(|e| {
            exports::outgoing_handler::ErrorCode::InternalError(Some(format!("{e}")))
        })?;
        if !url_allowed {
            (exports::outgoing_handler::ErrorCode::HttpRequestDenied);
        }
        let response = RESPONSES
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .remove(&url);
        match response {
            Some(r) => Ok(exports::types::FutureIncomingResponse::new(
                FutureIncomingResponse,
            )),
            None => Err(exports::outgoing_handler::ErrorCode::InternalError(Some(
                format!("unrecognized url: {url}"),
            ))),
        }
    }
}

impl crate::bindings::exports::fermyon::spin_wasi_virt::http_handler::Guest for Component {
    fn set_response(url: String, response: exports::types::OutgoingResponse) {
        RESPONSES
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .insert(url, response);
    }
}
