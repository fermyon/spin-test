use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;
use spin_sdk::key_value;

/// A simple Spin HTTP component.
#[http_component]
fn handle_example(_req: Request) -> anyhow::Result<impl IntoResponse> {
    let store = key_value::Store::open("example".into())?;
    let body = format!(
        r#""hello"={:?}"#,
        store.get("hello")?.as_deref().map(String::from_utf8_lossy)
    );
    Ok(Response::builder().status(200).body(body).build())
}
