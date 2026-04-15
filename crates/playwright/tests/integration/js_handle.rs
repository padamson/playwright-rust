// JSHandle — in-browser JavaScript value handle
// It is returned by frame.evaluate_handle_js() and jshandle.get_property().
//
// Tests cover:
// - json_value() returns the JSON-serializable value of the handle
// - get_property() returns a JSHandle for a named property
// - evaluate() evaluates JS with the handle as argument
// - dispose() releases the handle without error
//
// See: <https://playwright.dev/docs/api/class-jshandle>

use crate::test_server::TestServer;
use playwright_rs::protocol::{JSHandle, Playwright};

#[tokio::test]
async fn test_jshandle_json_value() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Obtain a JSHandle to an object via frame.evaluate_handle_js
    let frame = page.main_frame().await.expect("Failed to get main frame");
    let handle: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => ({ name: 'test', value: 42 })")
        .await
        .expect("Failed to get JSHandle");

    // json_value() should return the JSON-serializable value
    let value = handle.json_value().await.expect("Failed to get json_value");

    assert_eq!(value["name"], "test");
    assert_eq!(value["value"], 42);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_jshandle_get_property() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let frame = page.main_frame().await.expect("Failed to get main frame");
    let handle: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => ({ name: 'test', value: 42 })")
        .await
        .expect("Failed to get JSHandle");

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

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_jshandle_evaluate() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let frame = page.main_frame().await.expect("Failed to get main frame");
    let handle: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => ({ x: 10, y: 5 })")
        .await
        .expect("Failed to get JSHandle");

    // evaluate() evaluates expression with the handle as first argument
    let result: i64 = handle
        .evaluate("(obj) => obj.x + obj.y", None::<&()>)
        .await
        .expect("Failed to evaluate with handle");

    assert_eq!(result, 15);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_jshandle_dispose() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let frame = page.main_frame().await.expect("Failed to get main frame");
    let handle: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => ({ name: 'disposable' })")
        .await
        .expect("Failed to get JSHandle");

    // dispose() should succeed without error
    handle.dispose().await.expect("Failed to dispose JSHandle");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_jshandle_primitive_value() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let frame = page.main_frame().await.expect("Failed to get main frame");

    // JSHandle to a primitive number
    let handle: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => 42")
        .await
        .expect("Failed to get JSHandle for primitive");

    let value = handle
        .json_value()
        .await
        .expect("Failed to get json_value of primitive");

    assert_eq!(value, serde_json::json!(42));

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_jshandle_get_properties() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let frame = page.main_frame().await.expect("Failed to get main frame");
    let handle: std::sync::Arc<JSHandle> = frame
        .evaluate_handle_js("() => ({ a: 1, b: 2, c: 3 })")
        .await
        .expect("Failed to get JSHandle");

    // get_properties() returns a map of all enumerable properties
    let props = handle
        .get_properties()
        .await
        .expect("Failed to get properties");

    assert!(props.contains_key("a"), "Should contain key 'a'");
    assert!(props.contains_key("b"), "Should contain key 'b'");
    assert!(props.contains_key("c"), "Should contain key 'c'");
    assert_eq!(props.len(), 3, "Should have exactly 3 properties");

    // Each property should be a JSHandle with json_value() == its value
    let a_value = props["a"]
        .json_value()
        .await
        .expect("Failed to get json_value of 'a'");
    assert_eq!(a_value, serde_json::json!(1));

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
