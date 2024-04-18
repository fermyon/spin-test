use spin_test_sdk::{bindings::wasi::http, key_value, spin_test};

#[spin_test]
fn cache_hit() {
    let user_json = r#"{"id":123,"name":"Ryan"}"#;

    // Configure the test
    let key_value_config = key_value::Store::open("cache").unwrap();
    // Set state of the key-value store
    key_value_config.set("123", user_json.as_bytes()).unwrap();
    // Reset the call history
    key_value_config.reset_calls();

    make_request(user_json);

    // Assert the key-value store was queried
    assert_eq!(
        key_value_config.calls(),
        vec![key_value::Call::Get("123".to_owned())]
    );
}

#[spin_test]
fn cache_miss() {
    let user_json = r#"{"id":123,"name":"Ryan"}"#;

    // TODO:
    // http_handler::set_response("https://my.api.com?user_id=123", response);
    // Configure the test
    make_request(user_json);

    // Assert the key-value store was queried
    let key_value_config = key_value::Store::open("cache").unwrap();
    assert_eq!(
        key_value_config.calls(),
        vec![key_value::Call::Get("123".to_owned())]
    );
}

/// Actually perform the request against Spin
///
/// Asserts a 200 status code and the user JSON in the response body
fn make_request(user_json: &str) {
    // Perform the request
    let request = http::types::OutgoingRequest::new(http::types::Headers::new());
    request.set_path_with_query(Some("/?user_id=123")).unwrap();
    let response = spin_test_sdk::perform_request(request);

    // Assert response status and body
    assert_eq!(response.status(), 200);
    let body = response.body_as_string().unwrap();
    assert_eq!(body, user_json);
}
