use spin_test_sdk::{
    fermyon::spin::{key_value, sqlite},
    fermyon::spin_test::perform_request,
    fermyon::spin_test_virt::{key_value_calls, sqlite as virtualized_sqlite},
    spin_test,
    wasi::http,
};

#[spin_test]
fn cache_hit() {
    let user_json = r#"{"id":123,"name":"Ryan"}"#;

    // Configure the test
    let key_value_config = key_value::Store::open("cache").unwrap();
    // Set state of the key-value store
    key_value_config.set("123", user_json.as_bytes()).unwrap();
    // Reset the call history
    key_value_calls::reset_calls();

    // Set the expected response for the SQLite query
    virtualized_sqlite::set_response(
        "select name from users where user_id = ?;",
        &[sqlite::Value::Integer(123)],
        Ok(&sqlite::QueryResult {
            columns: vec!["name".into()],
            rows: vec![sqlite::RowResult {
                values: vec![sqlite::Value::Text("Ryan".into())],
            }],
        }),
    );

    // Perform the request
    let request = http::types::OutgoingRequest::new(http::types::Headers::new());
    request.set_path_with_query(Some("/?user_id=123")).unwrap();
    let response = perform_request(request);

    // Assert response status and body
    assert_eq!(response.status(), 200);
    let body = response.consume().unwrap().read_to_string().unwrap();
    assert_eq!(body, user_json);

    let calls = key_value_calls::calls()
        .into_iter()
        .find_map(|(key, value)| (key == "cache").then_some(value))
        .unwrap_or_default();
    assert_eq!(calls, vec![key_value_calls::Call::Get("123".to_owned())]);
}
