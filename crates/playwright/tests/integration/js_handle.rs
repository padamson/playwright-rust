use crate::test_server::TestServer;
use playwright_rs::protocol::JSHandle;

/// Exercises JSHandle methods in a single browser session:
/// json_value, get_property, evaluate, dispose, primitive value, get_properties.
#[tokio::test]
async fn test_jshandle_methods() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let frame = page.main_frame().await.expect("Failed to get main frame");

    // json_value() returns the JSON-serializable value of a handle
    let handle: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => ({ name: 'test', value: 42 })")
        .await
        .expect("Failed to get JSHandle");

    let value = handle.json_value().await.expect("Failed to get json_value");
    assert_eq!(value["name"], "test");
    assert_eq!(value["value"], 42);

    // get_property() returns a JSHandle for the named property
    let name_handle = handle
        .get_property("name")
        .await
        .expect("Failed to get property");
    let name_value = name_handle
        .json_value()
        .await
        .expect("Failed to get json_value of property");
    assert_eq!(name_value, "test");

    // evaluate() evaluates JS with the handle as first argument
    let math_handle: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => ({ x: 10, y: 5 })")
        .await
        .expect("Failed to get math JSHandle");
    let result: i64 = math_handle
        .evaluate("(obj) => obj.x + obj.y", None::<&()>)
        .await
        .expect("Failed to evaluate with handle");
    assert_eq!(result, 15);

    // dispose() releases the handle without error
    let disposable: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => ({ name: 'disposable' })")
        .await
        .expect("Failed to get disposable JSHandle");
    disposable
        .dispose()
        .await
        .expect("Failed to dispose JSHandle");

    // json_value() on a primitive value
    let prim: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => 42")
        .await
        .expect("Failed to get JSHandle for primitive");
    let prim_value = prim
        .json_value()
        .await
        .expect("Failed to get json_value of primitive");
    assert_eq!(prim_value, serde_json::json!(42));

    // get_properties() returns a map of all enumerable properties
    let obj_handle: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => ({ a: 1, b: 2, c: 3 })")
        .await
        .expect("Failed to get JSHandle");
    let props = obj_handle
        .get_properties()
        .await
        .expect("Failed to get properties");
    assert!(props.contains_key("a"), "Should contain key 'a'");
    assert!(props.contains_key("b"), "Should contain key 'b'");
    assert!(props.contains_key("c"), "Should contain key 'c'");
    assert_eq!(props.len(), 3, "Should have exactly 3 properties");
    let a_value = props["a"]
        .json_value()
        .await
        .expect("Failed to get json_value of 'a'");
    assert_eq!(a_value, serde_json::json!(1));

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
