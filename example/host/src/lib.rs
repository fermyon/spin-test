#[cfg(test)]
mod support;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use support::{Config, Runtime};

    use super::*;

    #[tokio::test]
    async fn app_works() {
        // Create a runtime
        let mut runtime = Runtime::create();

        // Load and instantiate the component
        let component_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../composition.wasm");
        let instance = support::load(&mut runtime, &component_path).await.unwrap();

        // Configure the test
        let config = Config::new(&mut runtime, &instance).unwrap();
        let user = r#"{"id":123,"name":"Ryan"}"#;

        // Set state of the key-value store
        let key_value_config = config.key_value_store(&mut runtime, "cache").await.unwrap();
        key_value_config
            .set(&mut runtime, "123", user.as_bytes())
            .await
            .unwrap();

        // Set a response for an HTTP request
        let handler = config.outbound_http_handler();
        let response = http::Response::builder()
            .status(200)
            .body(support::body::full(user.as_bytes().to_vec()))
            .unwrap();
        handler
            .set_response(&mut runtime, "https://my.api.com", response)
            .await
            .unwrap();

        // Hit the Spin app with an HTTP request
        let req = hyper::Request::builder()
            .uri("http://example.com:8080?user_id=123")
            .method(http::Method::GET)
            .body(support::body::empty())
            .unwrap();
        let response = support::perform_request(&mut runtime, &instance, req)
            .await
            .unwrap();

        use http_body_util::BodyExt;
        let (http::response::Parts { status, .. }, body) = response.into_parts();
        let body = body.collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert_eq!(status, http::StatusCode::OK, "Non-200 status code: {body}");
        assert_eq!(body, user);
    }
}
