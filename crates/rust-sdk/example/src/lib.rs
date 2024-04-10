#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use spin_test_sdk::Spin;

    #[tokio::test]
    async fn cache_hit_works() {
        // Create a runtime
        let mut spin = init_spin().await;

        // Configure the test
        let user = r#"{"id":123,"name":"Ryan"}"#;

        // Set state of the key-value store
        let mut key_value_config = spin.key_value_store("cache").await.unwrap();
        key_value_config.set("123", user.as_bytes()).await.unwrap();

        // Hit the Spin app with an HTTP request
        let (status, body) = make_request(&mut spin).await;

        let mut key_value_config = spin.key_value_store("cache").await.unwrap();
        let mut calls = key_value_config.calls().unwrap();
        let get_calls = calls.get_calls().await.unwrap();
        let set_calls = calls.set_calls().await.unwrap();

        assert_eq!(status, http::StatusCode::OK, "Non-200 status code: {body}");
        assert_eq!(
            get_calls,
            vec![spin_test_sdk::GetCall { key: "123".into() }],
        );
        assert!(set_calls.is_empty());
        assert_eq!(body, user);
    }

    #[tokio::test]
    async fn cache_miss_works() {
        // Create a runtime
        let mut spin = init_spin().await;

        // Configure the test
        let user = r#"{"id":123,"name":"Ryan"}"#;

        // Set a response for an HTTP request
        let mut handler = spin.outbound_http_handler();
        let response = http::Response::builder()
            .status(200)
            .body(spin_test_sdk::body::full(user.into()))
            .unwrap();
        handler
            .set_response("https://my.api.com?user_id=123", response)
            .await
            .unwrap();

        // Hit the Spin app with an HTTP request
        let (status, body) = make_request(&mut spin).await;

        let mut key_value_config = spin.key_value_store("cache").await.unwrap();
        let mut calls = key_value_config.calls().unwrap();
        let get_calls = calls.get_calls().await.unwrap();
        let set_calls = calls.set_calls().await.unwrap();

        assert_eq!(status, http::StatusCode::OK, "Non-200 status code: {body}");
        assert_eq!(
            get_calls,
            vec![spin_test_sdk::GetCall { key: "123".into() }],
        );
        assert_eq!(
            set_calls,
            vec![spin_test_sdk::SetCall {
                key: "123".into(),
                value: user.into(),
            }],
        );
        assert_eq!(body, user);
    }

    async fn init_spin() -> Spin {
        let component_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../composition.wasm");
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../guest/spin.toml");
        Spin::create(component_path, manifest_path).await.unwrap()
    }

    async fn make_request(spin: &mut Spin) -> (http::StatusCode, String) {
        let req = hyper::Request::builder()
            .uri("http://example.com:8080?user_id=123")
            .method(http::Method::GET)
            .body(spin_test_sdk::body::empty())
            .unwrap();
        let response = spin.perform_request(req).await.unwrap();

        use http_body_util::BodyExt;
        let (http::response::Parts { status, .. }, body) = response.into_parts();
        let body = body.collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&body).into_owned();
        (status, body)
    }
}
