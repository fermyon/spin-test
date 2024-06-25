// Some of the bindings code might not be currently used, but we're
// leaving it here since this code might change often and we may want
// to use some of the dead code in the near future.
#![allow(dead_code)]

use anyhow::anyhow;
use anyhow::Context as _;
use bindings::exports::fermyon::spin_wasi_virt::http_handler::ResponseHandler;
use bindings::{exports::wasi::http::types::HeaderError, VirtualizedApp};
use std::collections::HashMap;
use std::path::PathBuf;
use test_environment::http::Response;

mod bindings {
    wasmtime::component::bindgen!({
        world: "virtualized-app",
        path: "../host-wit",
        with: {
            "wasi:io": wasmtime_wasi::bindings::io,
            "wasi:clocks": wasmtime_wasi::bindings::clocks,
        }
    });
}

pub use bindings::VirtualizedAppImports;

/// The `spin-test` runtime
pub(crate) struct SpinTest {
    instance: VirtualizedApp,
    store: wasmtime::Store<super::StoreData>,
}

impl SpinTest {
    pub fn new(manifest: String, component_path: PathBuf) -> anyhow::Result<Self> {
        let mut engine_config = wasmtime::Config::new();
        engine_config.cache_config_load_default()?;
        let engine = wasmtime::Engine::new(&engine_config)?;

        let mut store = wasmtime::Store::new(&engine, super::StoreData::new(manifest));
        let mut linker = wasmtime::component::Linker::new(&engine);
        let component = spin_test::Component::from_file(component_path)?;
        let component = spin_test::virtualize_app(component).context("failed to virtualize app")?;

        let component = wasmtime::component::Component::new(&engine, component)?;
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;
        bindings::VirtualizedApp::add_to_linker(&mut linker, |x| x)?;

        let (instance, _) = bindings::VirtualizedApp::instantiate(&mut store, &component, &linker)?;

        Ok(Self { instance, store })
    }

    /// Make an HTTP request against the `spin-test` runtime
    pub fn make_http_request(
        &mut self,
        request: test_environment::http::Request<String>,
    ) -> anyhow::Result<test_environment::http::Response> {
        let request = to_outgoing_request(&self.instance, &mut self.store, request)
            .context("failed to create outgoing-request")?;
        let response = to_incoming_response(&self.instance, &mut self.store, request)
            .context("failed to get incoming-response")?;
        from_incoming_response(&mut self.store, response)
            .context("failed to convert to `Response` from incoming-response")
    }

    pub fn set_echo_response(&mut self, url: &str) -> anyhow::Result<()> {
        self.instance
            .fermyon_spin_wasi_virt_http_handler()
            .call_set_response(&mut self.store, url, ResponseHandler::Echo)
    }
}

impl test_environment::Runtime for SpinTest {
    fn error(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Convert a test_environment::http::Request into a wasi::http::types::IncomingRequest
fn to_outgoing_request<'a>(
    instance: &'a VirtualizedApp,
    store: &mut wasmtime::Store<super::StoreData>,
    request: test_environment::http::Request<String>,
) -> anyhow::Result<IncomingRequest<'a>> {
    let fields = Fields::new(instance, store)?;
    for (n, v) in request.headers {
        fields.append(store, &(*n).to_owned(), &(*v).to_owned().into_bytes())??;
    }
    let outgoing_request = OutgoingRequest::new(instance, store, fields)?;
    let method = match request.method {
        test_environment::http::Method::Get => bindings::exports::wasi::http::types::Method::Get,
        test_environment::http::Method::Post => bindings::exports::wasi::http::types::Method::Post,
        test_environment::http::Method::Put => bindings::exports::wasi::http::types::Method::Put,
        test_environment::http::Method::Patch => {
            bindings::exports::wasi::http::types::Method::Patch
        }
        test_environment::http::Method::Delete => {
            bindings::exports::wasi::http::types::Method::Delete
        }
    };
    outgoing_request
        .set_method(store, &method)?
        .map_err(|_| anyhow!("invalid request method"))?;
    outgoing_request
        .set_path_with_query(store, Some(request.path))?
        .map_err(|_| anyhow!("invalid request path"))?;
    // TODO: set the body
    IncomingRequest::new(instance, store, outgoing_request)
}

/// Call the incoming handler with the request and return the response
fn to_incoming_response<'a>(
    instance: &'a VirtualizedApp,
    store: &mut wasmtime::Store<super::StoreData>,
    request: IncomingRequest<'a>,
) -> anyhow::Result<IncomingResponse<'a>> {
    let (out, rx) = new_response(instance, &mut *store)
        .context("failed to create out-response and response-receiver")?;
    instance
        .wasi_http_incoming_handler()
        .call_handle(&mut *store, request.resource, out)
        .context("call to incoming-handler failed")?;
    rx.get(&mut *store)?.context("no response found")
}

/// Convert a wasi::http::types::IncomingResponse into a test_environment::http::Response
fn from_incoming_response(
    store: &mut wasmtime::Store<super::StoreData>,
    response: IncomingResponse,
) -> anyhow::Result<Response> {
    let status = response.status(store)?;
    let headers = response
        .headers(store)?
        .entries(store)?
        .into_iter()
        .map(|(k, v)| Ok((k, String::from_utf8(v)?)))
        .collect::<anyhow::Result<HashMap<_, _>>>()?;
    let body = response
        .consume(store)?
        .map_err(|_| anyhow!("response body already consumed"))?
        .stream(store)?
        .map_err(|_| anyhow!("response body stream already consumed"))?
        .blocking_read(store, u64::MAX)??;
    Ok(Response::full(status, headers, body))
}

struct Fields<'a> {
    guest: bindings::exports::wasi::http::types::GuestFields<'a>,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> Fields<'a> {
    pub fn new<T>(
        instance: &'a VirtualizedApp,
        store: &mut wasmtime::Store<T>,
    ) -> anyhow::Result<Self> {
        let guest = instance.wasi_http_types().fields();
        let resource = guest.call_constructor(store)?;
        Ok(Self { guest, resource })
    }

    pub fn append<T>(
        &self,
        store: &mut wasmtime::Store<T>,
        name: &String,
        value: &Vec<u8>,
    ) -> anyhow::Result<Result<(), HeaderError>> {
        self.guest.call_append(store, self.resource, name, value)
    }

    fn entries(
        &self,
        store: &mut wasmtime::Store<super::StoreData>,
    ) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
        self.guest.call_entries(store, self.resource)
    }
}

struct OutgoingRequest<'a> {
    guest: bindings::exports::wasi::http::types::GuestOutgoingRequest<'a>,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> OutgoingRequest<'a> {
    pub fn new<T>(
        instance: &'a VirtualizedApp,
        store: &mut wasmtime::Store<T>,
        fields: Fields,
    ) -> anyhow::Result<Self> {
        let guest = instance.wasi_http_types().outgoing_request();
        let resource = guest.call_constructor(store, fields.resource)?;
        Ok(Self { guest, resource })
    }

    pub fn set_method<T>(
        &self,
        store: &mut wasmtime::Store<T>,
        method: &bindings::exports::wasi::http::types::Method,
    ) -> anyhow::Result<Result<(), ()>> {
        self.guest.call_set_method(store, self.resource, method)
    }

    pub fn set_path_with_query<T>(
        &self,
        store: &mut wasmtime::Store<T>,
        path: Option<&str>,
    ) -> anyhow::Result<Result<(), ()>> {
        self.guest
            .call_set_path_with_query(store, self.resource, path)
    }
}

struct OutgoingResponse<'a> {
    instance: &'a VirtualizedApp,
    guest: bindings::exports::wasi::http::types::GuestOutgoingResponse<'a>,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> OutgoingResponse<'a> {
    pub fn new<T>(
        instance: &'a VirtualizedApp,
        store: &mut wasmtime::Store<T>,
        fields: Fields,
    ) -> anyhow::Result<Self> {
        let guest = instance.wasi_http_types().outgoing_response();
        let resource = guest.call_constructor(store, fields.resource)?;
        Ok(Self {
            instance,
            guest,
            resource,
        })
    }

    pub fn body<T>(
        &self,
        store: &mut wasmtime::Store<T>,
    ) -> anyhow::Result<Result<OutgoingBody, ()>> {
        let resource = match self.guest.call_body(store, self.resource)? {
            Ok(r) => r,
            Err(()) => return Ok(Err(())),
        };
        Ok(Ok(OutgoingBody {
            instance: self.instance,
            guest: self.instance.wasi_http_types().outgoing_body(),
            resource,
        }))
    }
}

struct IncomingRequest<'a> {
    #[allow(dead_code)]
    guest: bindings::exports::wasi::http::types::GuestIncomingRequest<'a>,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> IncomingRequest<'a> {
    fn new<T>(
        instance: &'a VirtualizedApp,
        store: &mut wasmtime::Store<T>,
        outgoing_request: OutgoingRequest,
    ) -> anyhow::Result<Self> {
        let guest = instance.fermyon_spin_wasi_virt_http_helper();
        let resource = guest.call_new_request(store, outgoing_request.resource, None)?;
        let guest = instance.wasi_http_types().incoming_request();
        Ok(Self { guest, resource })
    }
}

fn new_response<'a, T>(
    instance: &'a VirtualizedApp,
    store: &mut wasmtime::Store<T>,
) -> anyhow::Result<(wasmtime::component::ResourceAny, ResponseReceiver<'a>)> {
    let guest = instance.fermyon_spin_wasi_virt_http_helper();
    let (out_param, rx) = guest.call_new_response(store)?;
    let rx_guest = instance
        .fermyon_spin_wasi_virt_http_helper()
        .response_receiver();

    Ok((
        out_param,
        ResponseReceiver {
            instance,
            resource: rx,
            guest: rx_guest,
        },
    ))
}

struct ResponseReceiver<'a> {
    instance: &'a VirtualizedApp,
    guest: bindings::exports::fermyon::spin_wasi_virt::http_helper::GuestResponseReceiver<'a>,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> ResponseReceiver<'a> {
    fn get<T>(
        &self,
        store: &mut wasmtime::Store<T>,
    ) -> anyhow::Result<Option<IncomingResponse<'a>>> {
        let Some(resource) = self.guest.call_get(store, self.resource)? else {
            return Ok(None);
        };
        Ok(Some(IncomingResponse {
            instance: self.instance,
            guest: self.instance.wasi_http_types().incoming_response(),
            resource,
        }))
    }
}

struct IncomingResponse<'a> {
    instance: &'a VirtualizedApp,
    guest: bindings::exports::wasi::http::types::GuestIncomingResponse<'a>,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> IncomingResponse<'a> {
    fn status<T>(&self, store: &mut wasmtime::Store<T>) -> anyhow::Result<u16> {
        self.guest.call_status(store, self.resource)
    }

    fn headers<T>(&self, store: &mut wasmtime::Store<T>) -> anyhow::Result<Fields> {
        let fields = self.guest.call_headers(store, self.resource)?;
        Ok(Fields {
            guest: self.instance.wasi_http_types().fields(),
            resource: fields,
        })
    }

    fn consume<T>(
        &self,
        store: &mut wasmtime::Store<T>,
    ) -> wasmtime::Result<Result<IncomingBody<'a>, ()>> {
        let Ok(resource) = self.guest.call_consume(store, self.resource)? else {
            return Ok(Err(()));
        };
        let guest = self.instance.wasi_http_types().incoming_body();
        Ok(Ok(IncomingBody {
            instance: self.instance,
            guest,
            resource,
        }))
    }
}

struct IncomingBody<'a> {
    instance: &'a VirtualizedApp,
    guest: bindings::exports::wasi::http::types::GuestIncomingBody<'a>,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> IncomingBody<'a> {
    fn stream<T>(
        &self,
        store: &mut wasmtime::Store<T>,
    ) -> wasmtime::Result<Result<InputStream, ()>> {
        let Ok(resource) = self.guest.call_stream(store, self.resource)? else {
            return Ok(Err(()));
        };
        Ok(Ok(InputStream {
            instance: self.instance,
            resource,
        }))
    }
}

struct InputStream<'a> {
    instance: &'a VirtualizedApp,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> InputStream<'a> {
    fn blocking_read<T>(
        &self,
        store: &mut wasmtime::Store<T>,
        max_bytes: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, bindings::exports::wasi::io::streams::StreamError>> {
        self.instance
            .wasi_io_streams()
            .input_stream()
            .call_blocking_read(store, self.resource, max_bytes)
    }
}

struct OutgoingBody<'a> {
    instance: &'a VirtualizedApp,
    guest: bindings::exports::wasi::http::types::GuestOutgoingBody<'a>,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> OutgoingBody<'a> {
    fn write<T>(&self, store: &mut wasmtime::Store<T>) -> anyhow::Result<Result<OutputStream, ()>> {
        let stream = match self.guest.call_write(store, self.resource)? {
            Ok(s) => s,
            Err(()) => return Ok(Err(())),
        };
        Ok(Ok(OutputStream {
            instance: self.instance,
            resource: stream,
        }))
    }
}

struct OutputStream<'a> {
    instance: &'a VirtualizedApp,
    resource: wasmtime::component::ResourceAny,
}

impl<'a> OutputStream<'a> {
    fn blocking_write_and_flush<T>(
        &self,
        store: &mut wasmtime::Store<T>,
        data: &[u8],
    ) -> wasmtime::Result<Result<(), bindings::exports::wasi::io::streams::StreamError>> {
        self.instance
            .wasi_io_streams()
            .output_stream()
            .call_blocking_write_and_flush(store, self.resource, data)
    }
}
