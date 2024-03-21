use spin_sdk::http::{send, IntoResponse, Request, Response};
use spin_sdk::{http_component, key_value, llm};

/// A simple Spin HTTP component.
#[http_component]
async fn handle_example(_req: Request) -> anyhow::Result<impl IntoResponse> {
    let store = key_value::Store::open("example".into())?;
    let req = Request::get("https://example.com");
    let response: Response = send(req).await?;
    let inference = llm::infer(llm::InferencingModel::Llama2Chat, "Say hello")?.text;
    let body = format!(
        r#""hello"={:?}\ninference={inference}\nresponse={}"#,
        store.get("hello")?.as_deref().map(String::from_utf8_lossy),
        response.status()
    );
    Ok(Response::builder().status(200).body(body).build())
}
