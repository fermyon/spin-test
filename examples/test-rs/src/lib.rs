use spin_test_sdk::{
    bindings::fermyon::spin_wasi_virt::http_handler,
    bindings::{fermyon::spin_test_virt::key_value, wasi::http},
    spin_test,
};

#[spin_test]
fn cache_hit() {
    println!("In TEST: ENV_VAR={:?}", std::env::var("ENV_VAR"));
    let user_json = r#"{"id":123,"name":"Ryan"}"#;

    // Configure the test
    let key_value = key_value::Store::open("cache");
    // Set state of the key-value store
    key_value.set("123", user_json.as_bytes());

    make_request(user_json);

    // Assert the key-value store was queried
    assert_eq!(
        key_value.calls(),
        vec![key_value::Call::Get("123".to_owned())]
    );
}

#[spin_test]
fn cache_miss() {
    let user_json = r#"{"id":123,"name":"Ryan"}"#;

    let response = http::types::OutgoingResponse::new(http::types::Headers::new());
    response.write_body(user_json.as_bytes());
    http_handler::set_response("https://my.api.com?user_id=123", response);
    // Configure the test
    make_request(user_json);

    // Assert the key-value store was queried
    let key_value_config = key_value::Store::open("cache");
    assert_eq!(
        key_value_config.calls(),
        vec![
            key_value::Call::Get("123".to_owned()),
            key_value::Call::Set(("123".to_owned(), user_json.as_bytes().to_vec()))
        ]
    );
    assert_eq!(
        key_value_config.get("123").as_deref(),
        Some(user_json.as_bytes())
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
