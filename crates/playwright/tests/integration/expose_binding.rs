// Tests for:
// - BrowserContext::expose_function  (context-level, all pages get the binding)
// - BrowserContext::expose_binding   (same but with needsHandle: true)
// - Page::expose_function            (page-level, only that page gets the binding)
// - Page::expose_binding             (page-level with needsHandle: true)
//
// See: https://playwright.dev/docs/api/class-browsercontext#browser-context-expose-function
// See: https://playwright.dev/docs/api/class-page#page-expose-function

use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// BrowserContext::expose_function
// ---------------------------------------------------------------------------

/// A callback registered via context.expose_function is callable from any page
/// in the context and returns the callback's return value.
#[tokio::test]
async fn test_context_expose_function() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    // Register a simple add() function in the context
    context
        .expose_function("add", |args: Vec<serde_json::Value>| async move {
            let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
            serde_json::json!(a + b)
        })
        .await
        .expect("expose_function should succeed");

    let page = context.new_page().await.expect("new_page should succeed");

    // Call the exposed function from JS
    let result = page
        .evaluate_value("add(3, 7)")
        .await
        .expect("evaluate_value should succeed");

    assert_eq!(result, "10", "add(3, 7) should return 10, got: {result}");

    browser.close().await.expect("browser close");
}

/// A callback registered on a context is available in ALL pages created after
/// the registration.
#[tokio::test]
async fn test_context_expose_function_multiple_pages() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let call_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let call_count2 = call_count.clone();

    context
        .expose_function("countCall", move |_args: Vec<serde_json::Value>| {
            let count = call_count2.clone();
            async move {
                let mut c = count.lock().unwrap();
                *c += 1;
                serde_json::json!(*c)
            }
        })
        .await
        .expect("expose_function should succeed");

    let page1 = context.new_page().await.expect("page1");
    let page2 = context.new_page().await.expect("page2");

    let r1 = page1
        .evaluate_value("countCall()")
        .await
        .expect("evaluate page1");
    let r2 = page2
        .evaluate_value("countCall()")
        .await
        .expect("evaluate page2");

    assert_eq!(r1, "1", "first call should return 1, got: {r1}");
    assert_eq!(r2, "2", "second call should return 2, got: {r2}");

    browser.close().await.expect("browser close");
}

/// Expose a function that returns a string.
#[tokio::test]
async fn test_context_expose_function_string_return() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    context
        .expose_function("greet", |args: Vec<serde_json::Value>| async move {
            let name = args
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("World")
                .to_string();
            serde_json::json!(format!("Hello, {name}!"))
        })
        .await
        .expect("expose_function should succeed");

    let page = context.new_page().await.expect("new_page");

    let result = page
        .evaluate_value("greet('Playwright')")
        .await
        .expect("evaluate_value");

    assert_eq!(
        result, "Hello, Playwright!",
        "greet should return 'Hello, Playwright!', got: {result}"
    );

    browser.close().await.expect("browser close");
}

// ---------------------------------------------------------------------------
// BrowserContext::expose_binding
// ---------------------------------------------------------------------------

/// expose_binding works the same as expose_function but sends needsHandle: true
/// to the server. When needsHandle is true, the JS function only accepts a single
/// argument (a JSHandle). We verify that a single-argument call works.
#[tokio::test]
async fn test_context_expose_binding() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    context
        .expose_binding("double", |args: Vec<serde_json::Value>| async move {
            let x = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
            serde_json::json!(x * 2.0)
        })
        .await
        .expect("expose_binding should succeed");

    let page = context.new_page().await.expect("new_page");

    // With needsHandle: true, only one JS argument is allowed
    let result = page
        .evaluate_value("double(21)")
        .await
        .expect("evaluate_value");

    assert_eq!(result, "42", "double(21) should return 42, got: {result}");

    browser.close().await.expect("browser close");
}

// ---------------------------------------------------------------------------
// Page::expose_function
// ---------------------------------------------------------------------------

/// A callback registered via page.expose_function is callable from that page
/// but NOT from other pages in the same context.
#[tokio::test]
async fn test_page_expose_function() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let page = context.new_page().await.expect("new_page");

    page.expose_function("square", |args: Vec<serde_json::Value>| async move {
        let x = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
        serde_json::json!(x * x)
    })
    .await
    .expect("page expose_function should succeed");

    let result = page
        .evaluate_value("square(9)")
        .await
        .expect("evaluate_value");

    assert_eq!(result, "81", "square(9) should return 81, got: {result}");

    browser.close().await.expect("browser close");
}

/// A page-level binding is NOT available on a different page in the same context.
#[tokio::test]
async fn test_page_expose_function_isolated() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let page1 = context.new_page().await.expect("page1");
    let page2 = context.new_page().await.expect("page2");

    page1
        .expose_function("onlyOnPage1", |_args: Vec<serde_json::Value>| async move {
            serde_json::json!("from page1")
        })
        .await
        .expect("expose_function on page1");

    // page1 should have it
    let result = page1
        .evaluate_value("onlyOnPage1()")
        .await
        .expect("page1 evaluate");
    assert_eq!(result, "from page1", "page1 result: {result}");

    // page2 should NOT have it (evaluate should error or return undefined)
    let result2 = page2
        .evaluate_value("typeof onlyOnPage1")
        .await
        .expect("page2 typeof");
    assert_eq!(
        result2, "undefined",
        "page2 should not see onlyOnPage1, got: {result2}"
    );

    browser.close().await.expect("browser close");
}

// ---------------------------------------------------------------------------
// Page::expose_binding
// ---------------------------------------------------------------------------

/// page.expose_binding works the same as page.expose_function but sends
/// needsHandle: true to the server. With needsHandle: true, the JS function
/// only accepts a single argument.
#[tokio::test]
async fn test_page_expose_binding() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let page = context.new_page().await.expect("new_page");

    page.expose_binding("triple", |args: Vec<serde_json::Value>| async move {
        let x = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
        serde_json::json!(x * 3.0)
    })
    .await
    .expect("expose_binding should succeed");

    // With needsHandle: true, only a single JS argument is allowed
    let result = page
        .evaluate_value("triple(14)")
        .await
        .expect("evaluate_value");

    assert_eq!(result, "42", "triple(14) should return 42, got: {result}");

    browser.close().await.expect("browser close");
}
