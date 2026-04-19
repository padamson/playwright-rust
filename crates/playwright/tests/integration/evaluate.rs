use crate::test_server::TestServer;
use serde::{Deserialize, Serialize};

// ============================================================================
// Helper structs for typed evaluate() results
// ============================================================================

/// Struct for object results with properties
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ObjectResult {
    x: i32,
    y: i32,
}

/// Exercises evaluate_value() in a single browser session:
/// arithmetic, string, boolean, and fetch result.
#[tokio::test]
async fn test_evaluate_value_methods() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // evaluate_value() — simple arithmetic
    let result = page
        .evaluate_value("1 + 1")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "2");

    // evaluate_value() — string literal
    let result = page
        .evaluate_value("'hello'")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "hello");

    // evaluate_value() — boolean true
    let result = page
        .evaluate_value("true")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "true");

    // evaluate_value() — boolean false
    let result = page
        .evaluate_value("false")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "false");

    // evaluate_value() — fetch result (should succeed without routing)
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
    assert_eq!(result, "success");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Exercises evaluate() and evaluate_expression() in a single browser session:
/// numeric argument, string argument, object argument, no argument,
/// expression with no return value, array argument, and boolean argument.
#[tokio::test]
async fn test_evaluate_with_arguments() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // evaluate() with numeric argument — returns i64
    let arg = 5;
    let result: i64 = page
        .evaluate("(x) => x * 2", Some(&arg))
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, 10, "Expected numeric result 10");

    // evaluate() with string argument — returns String
    let arg = "hello";
    let result: String = page
        .evaluate("(s) => s.toUpperCase()", Some(&arg))
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "HELLO", "Expected uppercase string result");

    // evaluate() with object argument — using a struct
    let arg = ObjectResult { x: 10, y: 5 };
    let result: i32 = page
        .evaluate("(obj) => obj.x + obj.y", Some(&arg))
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, 15, "Expected sum of object properties");

    // evaluate() without argument (passing None) — returns i64
    let result: i64 = page
        .evaluate("() => 42", None::<&()>)
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, 42, "Expected numeric result 42");

    // evaluate_expression() — no return value
    let result = page.evaluate_expression("console.log('test')").await;
    assert!(result.is_ok(), "evaluate_expression should succeed");

    // evaluate() with array argument — returns i64
    let arg = vec![1, 2, 3, 4];
    let result: i64 = page
        .evaluate("(arr) => arr.reduce((a, b) => a + b, 0)", Some(&arg))
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, 10, "Expected sum of array elements");

    // evaluate() with boolean argument — returns bool
    let arg = true;
    let result: bool = page
        .evaluate("(b) => !b", Some(&arg))
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, !arg, "Expected inverted boolean result");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
