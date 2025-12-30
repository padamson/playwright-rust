// Integration tests for Page.evaluate_value() (Phase 5, Slice 4c)
//
// Tests JavaScript evaluation with return values

mod test_server;

use playwright_rs::protocol::Playwright;
use serde::{Deserialize, Serialize};
use test_server::TestServer;

mod common;

// ============================================================================
// Helper structs for typed evaluate() results
// ============================================================================

/// Struct for object results with properties
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ObjectResult {
    x: i32,
    y: i32,
}

#[tokio::test]
async fn test_evaluate_arithmetic() {
    common::init_tracing();
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

    // Test simple arithmetic
    let result = page
        .evaluate_value("1 + 1")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "2");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_string() {
    common::init_tracing();
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

    // Test string
    let result = page
        .evaluate_value("'hello'")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "hello");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_boolean() {
    common::init_tracing();
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

    // Test boolean
    let result = page
        .evaluate_value("true")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "true");

    let result = page
        .evaluate_value("false")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "false");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_fetch_result() {
    common::init_tracing();
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

    // Test fetch success/failure
    let result = page
        .evaluate_value(
            r#"
        fetch('/image.png')
            .then(() => 'success')
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");

    // Should succeed without routing
    assert_eq!(result, "success");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_with_argument() {
    common::init_tracing();
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

    // Test evaluate with numeric argument - returns an i64
    let arg = 5;
    let result: i64 = page
        .evaluate("(x) => x * 2", Some(&arg))
        .await
        .expect("Failed to evaluate");

    assert_eq!(result, 10, "Expected numeric result 10");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_with_string_argument() {
    common::init_tracing();
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

    // Test evaluate with string argument - returns a String
    let arg = "hello";
    let result: String = page
        .evaluate("(s) => s.toUpperCase()", Some(&arg))
        .await
        .expect("Failed to evaluate");

    assert_eq!(result, "HELLO", "Expected uppercase string result");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_with_object_argument() {
    common::init_tracing();
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

    // Test evaluate with object argument - using a struct
    let arg = ObjectResult { x: 10, y: 5 };
    let result: i32 = page
        .evaluate("(obj) => obj.x + obj.y", Some(&arg))
        .await
        .expect("Failed to evaluate");

    assert_eq!(result, 15, "Expected sum of object properties");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_without_argument() {
    common::init_tracing();
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

    // Test evaluate without argument (passing None) - returns i64
    let result: i64 = page
        .evaluate("() => 42", None::<&()>)
        .await
        .expect("Failed to evaluate");

    assert_eq!(result, 42, "Expected numeric result 42");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_expression_no_return() {
    common::init_tracing();
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

    // Test evaluate_expression (no return value)
    let result = page.evaluate_expression("console.log('test')").await;
    assert!(result.is_ok(), "evaluate_expression should succeed");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_with_array_argument() {
    common::init_tracing();
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

    // Test evaluate with array argument - returns i64
    let arg = vec![1, 2, 3, 4];
    let result: i64 = page
        .evaluate("(arr) => arr.reduce((a, b) => a + b, 0)", Some(&arg))
        .await
        .expect("Failed to evaluate");

    assert_eq!(result, 10, "Expected sum of array elements");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_with_boolean_argument() {
    common::init_tracing();
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

    // Test evaluate with boolean argument - returns bool
    let arg = true;
    let result: bool = page
        .evaluate("(b) => !b", Some(&arg))
        .await
        .expect("Failed to evaluate");

    assert_eq!(result, !arg, "Expected inverted boolean result");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
