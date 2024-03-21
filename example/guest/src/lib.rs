use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::{http_component, key_value, llm};

/// A simple Spin HTTP component.
#[http_component]
fn handle_example(_req: Request) -> anyhow::Result<impl IntoResponse> {
    let store = key_value::Store::open("example".into())?;
    let inference = llm::infer(llm::InferencingModel::Llama2Chat, "Say hello")?.text;
    let body = format!(
        r#""hello"={:?}\ninference={inference}"#,
        store.get("hello")?.as_deref().map(String::from_utf8_lossy)
    );
    Ok(Response::builder().status(200).body(body).build())
}
