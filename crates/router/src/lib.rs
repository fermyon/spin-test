#[allow(warnings)]
mod bindings;

use anyhow::Context as _;
use bindings::{
    exports::wasi::http::incoming_handler::Guest,
    wasi::http::incoming_handler::handle as downstream,
    wasi::http::types::{
        ErrorCode, Headers, IncomingRequest, OutgoingRequest, ResponseOutparam, Scheme,
    },
};

struct Component;

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let mut manifest: spin_manifest::schema::v2::AppManifest =
            toml::from_str(&bindings::get_manifest()).unwrap();
        spin_manifest::normalize::normalize_manifest(&mut manifest);
        let routes = manifest
            .triggers
            .get("http")
            .unwrap()
            .iter()
            .map(|trigger| {
                let spin_manifest::schema::v2::ComponentSpec::Reference(comp) =
                    trigger.component.as_ref().unwrap()
                else {
                    todo!()
                };
                (
                    trigger
                        .config
                        .get("route")
                        .and_then(|route| route.as_str())
                        .unwrap(),
                    comp.to_string(),
                )
            })
            .collect::<Vec<_>>();
        let path_with_query = request
            .path_with_query()
            .unwrap_or_else(|| String::from("/"));
        let path = path_with_query
            .split_once('?')
            .map(|(path, _)| path)
            .unwrap_or(&path_with_query);
        let routing = routes
            .iter()
            .find(|(route, _)| path.starts_with(route) || *route == "/...");
        if let Some((route, component_id)) = routing {
            let request = match apply_request_transformations(request, (*route).to_owned()) {
                Ok(request) => request,
                Err(e) => {
                    ResponseOutparam::set(
                        response_out,
                        Err(ErrorCode::InternalError(Some(e.to_string()))),
                    );
                    return;
                }
            };
            bindings::set_component_id(component_id);
            downstream(request, response_out)
        } else {
            ResponseOutparam::set(
                response_out,
                Err(ErrorCode::InternalError(
                    format!("no route found in spin.toml manifest for request path '{path}'")
                        .into(),
                )),
            )
        }
    }
}

/// Apply any request transformations needed for the given route.
fn apply_request_transformations(
    request: IncomingRequest,
    raw_route: String,
) -> anyhow::Result<IncomingRequest> {
    let headers_to_add = calculate_default_headers(&request, raw_route)
        .context("could not calculate default headers to for request")?
        .into_iter()
        .flat_map(|(k, v)| {
            k.iter()
                .map(move |s| (s.to_string(), v.clone().into_bytes()))
        })
        .chain(request.headers().entries());
    let headers = Headers::new();
    for (key, value) in headers_to_add {
        headers.append(&key, &value).unwrap();
    }
    let new = OutgoingRequest::new(headers);
    let _ = new.set_scheme(request.scheme().as_ref());
    let _ = new.set_authority(request.authority().as_deref());
    let _ = new.set_method(&request.method());
    let _ = new.set_path_with_query(request.path_with_query().as_deref());
    Ok(bindings::new_request(new, Some(request.consume().unwrap())))
}

const FULL_URL: &[&str] = &["SPIN-FULL-URL", "X-FULL-URL"];
const PATH_INFO: &[&str] = &["SPIN-PATH-INFO", "PATH-INFO"];
const MATCHED_ROUTE: &[&str] = &["SPIN-MATCHED-ROUTE", "X-MATCHED-ROUTE"];
const COMPONENT_ROUTE: &[&str] = &["SPIN-COMPONENT-ROUTE", "X-COMPONENT-ROUTE"];
const RAW_COMPONENT_ROUTE: &[&str] = &["SPIN-RAW-COMPONENT-ROUTE", "X-RAW-COMPONENT-ROUTE"];
const BASE_PATH: &[&str] = &["SPIN-BASE-PATH", "X-BASE-PATH"];
const CLIENT_ADDR: &[&str] = &["SPIN-CLIENT-ADDR", "X-CLIENT-ADDR"];
/// Calculate the default headers for the given request.
fn calculate_default_headers<'a>(
    req: &IncomingRequest,
    raw_route: String,
) -> anyhow::Result<Vec<(&'a [&'a str], String)>> {
    let mut res = vec![];
    // TODO: calculate base path from manifest
    let base = "/".to_owned();
    let abs_path = req.path_with_query().unwrap_or_else(|| String::from("/"));
    let scheme = req.scheme();
    let scheme = match scheme.as_ref().unwrap_or(&Scheme::Https) {
        Scheme::Http => "http",
        Scheme::Https => "https",
        Scheme::Other(s) => s,
    };
    let host = req
        .headers()
        .get(&"host".to_owned())
        .into_iter()
        .find(|v| !v.is_empty())
        .map(String::from_utf8)
        .transpose()
        .context("expected 'host' header to be UTF-8 encoded but it was not")?
        .unwrap_or_else(|| "localhost".to_owned());

    let matched_route =
        spin_http::routes::RoutePattern::sanitize_with_base(base.clone(), raw_route.clone());

    let path_info = spin_http::routes::RoutePattern::from(base.clone(), raw_route.clone())
        .relative(&abs_path)?;
    let full_url = format!("{}://{}{}", scheme, host, abs_path);
    let component_route = raw_route
        .strip_suffix("/...")
        .unwrap_or(&raw_route)
        .to_owned();

    res.push((PATH_INFO, path_info));
    res.push((FULL_URL, full_url));
    res.push((MATCHED_ROUTE, matched_route));
    res.push((BASE_PATH, base));
    res.push((RAW_COMPONENT_ROUTE, raw_route));
    res.push((COMPONENT_ROUTE, component_route));
    res.push((CLIENT_ADDR, "127.0.0.1:0".to_owned()));

    Ok(res)
}

bindings::export!(Component with_types_in bindings);
