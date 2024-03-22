#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use spin_test_sdk::Spin;

    #[tokio::test]
    async fn app_works() {
        // Create a runtime
        let component_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../composition.wasm");
        let mut spin = Spin::create(component_path).await.unwrap();

        // Configure the test
        let user = r#"{"id":123,"name":"Ryan"}"#;

        // Set state of the key-value store
        let mut key_value_config = spin.key_value_store("cache").await.unwrap();
        key_value_config.set("123", user.as_bytes()).await.unwrap();

        // Set a response for an HTTP request
        let mut handler = spin.outbound_http_handler();
        let response = http::Response::builder()
            .status(200)
            .body(spin_test_sdk::body::full(user.into()))
            .unwrap();
        handler
            .set_response("https://my.api.com", response)
            .await
            .unwrap();

        // Hit the Spin app with an HTTP request
        let req = hyper::Request::builder()
            .uri("http://example.com:8080?user_id=123")
            .method(http::Method::GET)
            .body(spin_test_sdk::body::empty())
            .unwrap();
        let response = spin.perform_request(req).await.unwrap();

        use http_body_util::BodyExt;
        let (http::response::Parts { status, .. }, body) = response.into_parts();
        let body = body.collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert_eq!(status, http::StatusCode::OK, "Non-200 status code: {body}");
        assert_eq!(body, user);
    }
}
