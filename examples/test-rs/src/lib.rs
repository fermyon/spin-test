use spin_test_sdk::{
    http::read_body,
    incoming_handler, key_value, key_value_calls, new_request, new_response, spin_test,
    wit::fermyon::{spin::sqlite::RowResult, spin_test_virt::sqlite},
    Headers, OutgoingRequest,
};

#[spin_test]
fn simple_kv_test1() {
    // Configure the test
    let user = r#"{"id":123,"name":"Ryan"}"#;

    let key_value_config = key_value::Store::open("cache").unwrap();
    // Set state of the key-value store
    key_value_config.set("123", user.as_bytes()).unwrap();
    key_value_calls::reset_calls();
    sqlite::set_response(
        "select name from users where user_id = ?;",
        &[sqlite::Value::Integer(123)],
        Ok(&sqlite::QueryResult {
            columns: vec!["name".into()],
            rows: vec![RowResult {
                values: vec![sqlite::Value::Text("Ryan".into())],
            }],
        }),
    );

    let request = OutgoingRequest::new(Headers::new());
    request.set_path_with_query(Some("/?user_id=123")).unwrap();
    let request = new_request(request);
    let (response_out, response_receiver) = new_response();
    incoming_handler::handle(request, response_out);
    let response = response_receiver.get().unwrap();
    assert_eq!(response.status(), 200);

    let mut body = String::new();
    read_body(response.consume().unwrap(), |buffer| {
        body.push_str(&String::from_utf8(buffer).unwrap())
    })
    .unwrap();
    assert_eq!(body, user);

    let calls = key_value_calls::calls()
        .into_iter()
        .find_map(|(key, value)| (key == "cache").then_some(value))
        .unwrap_or_default();
    assert_eq!(calls, vec![key_value_calls::Call::Get("123".to_owned())]);
}
