#[allow(warnings)]
mod bindings;

use bindings::{
    exports::wasi::http::incoming_handler::{IncomingRequest, ResponseOutparam},
    wasi::http::types::ErrorCode,
};

use crate::bindings::{
    exports::wasi::http::incoming_handler::Guest,
    wasi::http::incoming_handler::handle as downstream,
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
        let component_id = routes
            .iter()
            .find(|(route, _)| path.starts_with(route) || *route == "/...")
            .map(|(_, comp)| comp);
        if let Some(component_id) = component_id {
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

bindings::export!(Component with_types_in bindings);
