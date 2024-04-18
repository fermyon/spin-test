use spin_sdk::http::{send, IntoResponse, Request, Response};
use spin_sdk::sqlite::Value;
use spin_sdk::{http_component, key_value, redis, sqlite, variables};

/// A simple Spin HTTP component.
#[http_component]
async fn handle_example(req: Request) -> anyhow::Result<impl IntoResponse> {
    let redis = redis::Connection::open("redis://redis:6379")?;
    redis.set("key", &"value".to_owned().into_bytes())?;
    println!("Redis Value: {:?}", redis.get("key")?);

    let cache_name = variables::get("cache_name")?;
    let store = key_value::Store::open(&cache_name)?;
    let query: Query = serde_qs::from_str(req.query())?;

    let sqlite = sqlite::Connection::open("database")?;
    sqlite.execute(
        "select name from users where user_id = ?;",
        &[Value::Integer(query.user_id as i64)],
    )?;

    let cache = store.get(&query.user_id.to_string())?;
    let user: User = match cache {
        Some(hit) => serde_json::from_slice(&hit)?,
        None => {
            let req = Request::get(&format!("https://my.api.com?user_id={}", query.user_id));
            let response: Response = send(req).await?;
            let user = serde_json::from_slice(&response.body())?;
            store.set(&query.user_id.to_string(), &serde_json::to_vec(&user)?)?;
            user
        }
    };

    let body = serde_json::to_string(&user)?;
    Ok(Response::builder().status(200).body(body).build())
}

#[derive(serde::Deserialize)]
struct Query {
    user_id: usize,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct User {
    id: usize,
    name: String,
}
