// Integration tests for route.fallback() and page.unroute/unroute_all

use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_route_fallback_basic() {
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

    // Set up route that calls fallback (should send request through to network)
    page.route("**/*", |route| async move { route.fallback(None).await })
        .await
        .expect("Failed to set up route");

    let response = page
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    assert_eq!(
        response.status(),
        200,
        "Fallback should allow request through"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_fallback_handler_chaining() {
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

    let handler_order = Arc::new(Mutex::new(Vec::<String>::new()));

    // First handler (registered first, checked last due to reverse iteration)
    let order1 = handler_order.clone();
    page.route("**/*", move |route| {
        let order = order1.clone();
        async move {
            order.lock().unwrap().push("handler1".to_string());
            route.continue_(None).await
        }
    })
    .await
    .expect("Failed to set up route 1");

    // Second handler (registered last, checked first)
    // This one calls fallback(), so handler1 should also run
    let order2 = handler_order.clone();
    page.route("**/*", move |route| {
        let order = order2.clone();
        async move {
            order.lock().unwrap().push("handler2".to_string());
            route.fallback(None).await
        }
    })
    .await
    .expect("Failed to set up route 2");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let order = handler_order.lock().unwrap().clone();
    assert!(
        order.contains(&"handler2".to_string()),
        "Handler 2 (last registered) should run first"
    );
    assert!(
        order.contains(&"handler1".to_string()),
        "Handler 1 should run after handler 2 calls fallback()"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_unroute_removes_handler() {
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

    // Set up route to abort all image requests
    page.route("**/*.png", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Verify abort works
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
    assert_eq!(result, "failed", "Image should be aborted");

    // Now unroute the pattern
    page.unroute("**/*.png").await.expect("Failed to unroute");

    // Fetch should now succeed (or at least not be aborted by our handler)
    // Note: the server may 404, but the fetch itself shouldn't be aborted
    let result = page
        .evaluate_value(
            r#"
        fetch('/image.png')
            .then(r => 'status:' + r.status)
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");
    assert!(
        result.to_string().starts_with("status:"),
        "After unroute, fetch should not be aborted (got: {})",
        result
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_unroute_all_clears_all() {
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

    // Set up multiple abort routes
    page.route("**/*.png", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up route 1");
    page.route("**/*.css", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up route 2");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Verify abort works
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
    assert_eq!(result, "failed", "Image should be aborted");

    // Clear all routes
    page.unroute_all(None).await.expect("Failed to unroute all");

    // Both types should now pass through
    let result = page
        .evaluate_value(
            r#"
        fetch('/image.png')
            .then(r => 'status:' + r.status)
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");
    assert!(
        result.to_string().starts_with("status:"),
        "After unroute_all, png fetch should not be aborted (got: {})",
        result
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
