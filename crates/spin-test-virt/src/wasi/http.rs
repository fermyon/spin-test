use std::{
    borrow::{Borrow, BorrowMut},
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub use crate::bindings::exports::wasi::http as exports;
pub use crate::bindings::wasi::http as imports;

use crate::Component;

use super::io;

impl exports::types::Guest for Component {
    type IncomingRequest = IncomingRequest;
    type OutgoingResponse = OutgoingResponse;

    type OutgoingRequest = OutgoingRequest;
    type IncomingResponse = IncomingResponse;

    type OutgoingBody = OutgoingBody;
    type IncomingBody = IncomingBody;

    type Fields = Fields;
    type RequestOptions = RequestOptions;
    type ResponseOutparam = ResponseOutparam;
    type FutureIncomingResponse = FutureIncomingResponse;
    type FutureTrailers = FutureTrailers;

    fn http_error_code(
        err: io::exports::error::ErrorBorrow<'_>,
    ) -> Option<exports::types::ErrorCode> {
        None
    }
}

pub struct IncomingRequest {
    pub method: exports::types::Method,
    pub scheme: Option<exports::types::Scheme>,
    pub authority: Option<String>,
    pub path_with_query: Option<String>,
    pub headers: Fields,
    pub body: Consumable<IncomingBody>,
}

/// A `Result` type where `Err` is the consumed value.
#[derive(Clone, Debug)]
pub struct Consumable<T> {
    value: T,
    consumed: Cell<bool>,
}

impl<T> Consumable<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            consumed: Cell::new(false),
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Consumable<U> {
        Consumable {
            value: f(self.value),
            consumed: self.consumed,
        }
    }
}

impl<T: Clone> Consumable<T> {
    pub fn consume(&self) -> Result<T, ()> {
        if self.consumed.get() {
            Err(())
        } else {
            self.consumed.set(true);
            Ok(self.value.clone())
        }
    }

    pub fn unconsume(&self) -> Self {
        let new = self.clone();
        new.consumed.set(false);
        new
    }
}

impl<T> From<T> for Consumable<T> {
    fn from(value: T) -> Self {
        Consumable::new(value)
    }
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
        Ok(exports::types::IncomingBody::new(self.body.consume()?))
    }
}

#[derive(Clone, Debug)]
pub struct OutgoingResponse {
    pub status_code: Cell<exports::types::StatusCode>,
    pub headers: Fields,
    pub body: Consumable<OutgoingBody>,
}

impl exports::types::GuestOutgoingResponse for OutgoingResponse {
    fn new(headers: exports::types::Headers) -> Self {
        Self {
            status_code: Cell::new(200),
            headers: headers.into_inner(),
            body: Consumable::new(OutgoingBody(io::Buffer::empty())),
        }
    }

    fn status_code(&self) -> exports::types::StatusCode {
        self.status_code.get()
    }

    fn set_status_code(&self, status_code: exports::types::StatusCode) -> Result<(), ()> {
        // TODO: check that status code is valid
        self.status_code.set(status_code);
        Ok(())
    }

    fn headers(&self) -> exports::types::Headers {
        exports::types::Headers::new(self.headers.clone())
    }

    fn body(&self) -> Result<exports::types::OutgoingBody, ()> {
        Ok(exports::types::OutgoingBody::new(self.body.consume()?))
    }
}

pub struct OutgoingRequest {
    pub method: RefCell<exports::types::Method>,
    pub scheme: RefCell<Option<exports::types::Scheme>>,
    pub authority: RefCell<Option<String>>,
    pub path_with_query: RefCell<Option<String>>,
    pub headers: Fields,
    body: Consumable<OutgoingBody>,
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
            body: Consumable::new(OutgoingBody(io::Buffer::empty())),
        }
    }

    fn body(&self) -> Result<exports::types::OutgoingBody, ()> {
        let body = self.body.consume()?;
        Ok(exports::types::OutgoingBody::new(body))
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

pub struct IncomingResponse {
    pub status: exports::types::StatusCode,
    pub headers: Fields,
    pub body: Consumable<IncomingBody>,
}

impl exports::types::GuestIncomingResponse for IncomingResponse {
    fn status(&self) -> exports::types::StatusCode {
        self.status
    }

    fn headers(&self) -> exports::types::Headers {
        exports::types::Headers::new(self.headers.clone())
    }

    fn consume(&self) -> Result<exports::types::IncomingBody, ()> {
        Ok(exports::types::IncomingBody::new(self.body.consume()?))
    }
}

#[derive(Clone, Debug)]
pub struct OutgoingBody(io::Buffer);

impl exports::types::GuestOutgoingBody for OutgoingBody {
    fn write(&self) -> Result<io::exports::streams::OutputStream, ()> {
        Ok(io::exports::streams::OutputStream::new(
            io::OutputStream::Buffered(self.0.clone()),
        ))
    }

    fn finish(
        this: exports::types::OutgoingBody,
        trailers: Option<exports::types::Trailers>,
    ) -> Result<(), exports::types::ErrorCode> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct IncomingBody(io::Buffer);

impl IncomingBody {
    pub fn new(buffer: io::Buffer) -> Self {
        Self(buffer)
    }

    pub fn empty() -> Self {
        Self::new(io::Buffer::empty())
    }
}

impl exports::types::GuestIncomingBody for IncomingBody {
    fn stream(&self) -> Result<io::exports::streams::InputStream, ()> {
        Ok(io::exports::streams::InputStream::new(
            io::InputStream::Buffered(self.0.clone()),
        ))
    }

    fn finish(this: exports::types::IncomingBody) -> exports::types::FutureTrailers {
        exports::types::FutureTrailers::new(FutureTrailers)
    }
}

impl From<OutgoingBody> for IncomingBody {
    fn from(o: OutgoingBody) -> Self {
        Self(o.0)
    }
}

impl<T> From<T> for IncomingBody
where
    T: Into<io::Buffer>,
{
    fn from(t: T) -> Self {
        Self(t.into())
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
        let mut fields: HashMap<String, Vec<Vec<u8>>> = HashMap::new();
        for (k, v) in entries {
            fields.entry(k).or_default().push(v);
        }
        let fields = Fields {
            fields: RefCell::new(fields),
        };
        Ok(exports::types::Fields::new(fields))
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

pub struct ResponseOutparam(
    pub Arc<Mutex<Option<Result<exports::types::OutgoingResponse, exports::types::ErrorCode>>>>,
);

impl exports::types::GuestResponseOutparam for ResponseOutparam {
    fn set(
        mut param: exports::types::ResponseOutparam,
        response: Result<exports::types::OutgoingResponse, exports::types::ErrorCode>,
    ) {
        let inner: &mut ResponseOutparam = param.get_mut();
        *inner.0.lock().unwrap() = Some(response);
    }
}

pub struct FutureIncomingResponse(
    RefCell<Option<Result<IncomingResponse, exports::types::ErrorCode>>>,
);

impl FutureIncomingResponse {
    pub fn new(response: Result<IncomingResponse, exports::types::ErrorCode>) -> Self {
        Self(RefCell::new(Some(response)))
    }
}

impl exports::types::GuestFutureIncomingResponse for FutureIncomingResponse {
    fn subscribe(&self) -> io::exports::poll::Pollable {
        io::exports::poll::Pollable::new(io::Pollable::Virtualized)
    }

    fn get(
        &self,
    ) -> Option<Result<Result<exports::types::IncomingResponse, exports::types::ErrorCode>, ()>>
    {
        Some(
            self.0
                .borrow_mut()
                .take()
                .map(|s| s.map(exports::types::IncomingResponse::new))
                .ok_or(()),
        )
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
            Some(r) => {
                let r: OutgoingResponse = r.into_inner();
                Ok(exports::types::FutureIncomingResponse::new(
                    FutureIncomingResponse::new(Ok(IncomingResponse {
                        status: r.status_code.get(),
                        headers: r.headers,
                        body: r.body.unconsume().map(Into::into),
                    })),
                ))
            }
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
