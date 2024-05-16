#[allow(warnings)]
mod bindings;

use std::fmt::Display;

use anyhow::Context as _;
use bindings::{
    exports::wasi::http::incoming_handler::Guest,
    wasi::http::incoming_handler::handle as downstream,
    wasi::http::types::{
        ErrorCode, Headers, IncomingRequest, OutgoingRequest, ResponseOutparam, Scheme,
    },
};
use spin_http::routes::RouteMatch;

/// Print to the standard output.
///
/// We can't use `std::println!` because it's not available in the wasm32-unknown-unknown target.
#[macro_export]
macro_rules! println {
    ($($tt:tt)*) => {
        let stdout = $crate::bindings::wasi::cli::stdout::get_stdout();
        stdout
            .blocking_write_and_flush(format!("{}\n", format_args!($($tt)*)).as_bytes())
            .unwrap();
    };
}

struct Component;

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let mut manifest: spin_manifest::schema::v2::AppManifest =
            toml::from_str(&bindings::get_manifest()).unwrap();
        spin_manifest::normalize::normalize_manifest(&mut manifest);
        let base = manifest
            .application
            .trigger_global_configs
            .get("http")
            .and_then(|c| c.get("base").and_then(|v| v.as_str()))
            .unwrap_or("/");
        let route_match = match find_matching_route(&manifest, &request, base) {
            Err(e) => {
                set_error_response(response_out, e);
                return;
            }
            Ok(RoutingResult::RouteFound(route_match)) => route_match,
            Ok(RoutingResult::RouteNotFound) => {
                ResponseOutparam::set(response_out, Err(ErrorCode::DestinationNotFound));
                return;
            }
        };
        let request = match apply_request_transformations(request, base, &route_match) {
            Ok(request) => request,
            Err(e) => {
                set_error_response(response_out, e);
                return;
            }
        };
        bindings::set_component_id(route_match.component_id());
        downstream(request, response_out)
    }
}

fn set_error_response(response_out: ResponseOutparam, message: impl Display) {
    ResponseOutparam::set(
        response_out,
        Err(ErrorCode::InternalError(Some(message.to_string()))),
    );
}

enum RoutingResult {
    RouteFound(RouteMatch),
    RouteNotFound,
}

fn find_matching_route(
    manifest: &spin_manifest::schema::v2::AppManifest,
    request: &IncomingRequest,
    base: &str,
) -> anyhow::Result<RoutingResult> {
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
                comp.to_string(),
                trigger
                    .config
                    .get("route")
                    .and_then(|route| route.as_str())
                    .unwrap()
                    .into(),
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
    let (router, _) =
        spin_http::routes::Router::build(base, routes.iter().map(|(c, t)| (c.as_str(), t)))?;
    router
        .route(path)
        .map(RoutingResult::RouteFound)
        .or(Ok(RoutingResult::RouteNotFound))
}

/// Apply any request transformations needed for the given route.
fn apply_request_transformations(
    request: IncomingRequest,
    base: &str,
    route_match: &RouteMatch,
) -> anyhow::Result<IncomingRequest> {
    let headers_to_add = calculate_default_headers(&request, base, route_match)
        .context("could not calculate default headers to for request")?
        .into_iter()
        .flat_map(|(k, v)| {
            k.into_iter()
                .map(move |s| (s.to_string(), v.clone().into_bytes()))
        })
        .chain(request.headers().entries());
    let headers = Headers::new();
    for (key, value) in headers_to_add {
        headers.append(&key, &value).unwrap();
    }
    let new = OutgoingRequest::new(headers);
    // Make sure that the scheme and authority are set as the Spin runtime does this
    let _ = new.set_scheme(request.scheme().as_ref().or(Some(&Scheme::Http)));
    let _ = new.set_authority(request.authority().as_deref().or(Some("localhost")));
    let _ = new.set_method(&request.method());
    let _ = new.set_path_with_query(request.path_with_query().as_deref().or(Some("/")));
    Ok(bindings::new_request(new, Some(request.consume().unwrap())))
}

const FULL_URL: [&str; 2] = ["SPIN_FULL_URL", "X_FULL_URL"];
const PATH_INFO: [&str; 2] = ["SPIN_PATH_INFO", "PATH_INFO"];
const MATCHED_ROUTE: [&str; 2] = ["SPIN_MATCHED_ROUTE", "X_MATCHED_ROUTE"];
const COMPONENT_ROUTE: [&str; 2] = ["SPIN_COMPONENT_ROUTE", "X_COMPONENT_ROUTE"];
const RAW_COMPONENT_ROUTE: [&str; 2] = ["SPIN_RAW_COMPONENT_ROUTE", "X_RAW_COMPONENT_ROUTE"];
const BASE_PATH: [&str; 2] = ["SPIN_BASE_PATH", "X_BASE_PATH"];
const CLIENT_ADDR: [&str; 2] = ["SPIN_CLIENT_ADDR", "X_CLIENT_ADDR"];

fn calculate_default_headers(
    req: &IncomingRequest,
    base: &str,
    route_match: &RouteMatch,
) -> anyhow::Result<Vec<([String; 2], String)>> {
    fn owned(strs: &[&'static str; 2]) -> [String; 2] {
        let convert = |s: &str| s.to_owned().replace('_', "-");
        [convert(strs[0]), convert(strs[1])]
    }

    let owned_full_url = owned(&FULL_URL);
    let owned_path_info = owned(&PATH_INFO);
    let owned_matched_route = owned(&MATCHED_ROUTE);
    let owned_component_route = owned(&COMPONENT_ROUTE);
    let owned_raw_component_route = owned(&RAW_COMPONENT_ROUTE);
    let owned_base_path = owned(&BASE_PATH);
    let owned_client_addr = owned(&CLIENT_ADDR);

    let mut res = vec![];

    let abs_path = req.path_with_query().unwrap_or_else(|| String::from("/"));
    let path_info = route_match.trailing_wildcard();

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

    let full_url = format!("{}://{}{}", scheme, host, abs_path);

    res.push((owned_path_info, path_info));
    res.push((owned_full_url, full_url));
    res.push((owned_matched_route, route_match.based_route().to_string()));

    res.push((owned_base_path, base.to_string()));
    res.push((
        owned_raw_component_route,
        route_match.raw_route().to_string(),
    ));
    res.push((owned_component_route, route_match.raw_route_or_prefix()));
    res.push((owned_client_addr, "127.0.0.1:0".to_owned()));

    for (wild_name, wild_value) in route_match.named_wildcards() {
        let wild_header = format!("SPIN_PATH_MATCH_{}", wild_name.to_ascii_uppercase());
        let wild_wagi_header = format!("X_PATH_MATCH_{}", wild_name.to_ascii_uppercase());
        res.push(([wild_header, wild_wagi_header], wild_value.clone()));
    }

    Ok(res)
}

bindings::export!(Component with_types_in bindings);
