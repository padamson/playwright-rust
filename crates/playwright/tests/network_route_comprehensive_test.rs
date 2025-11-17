// Comprehensive integration tests for Network Routing (Phase 5, Slice 4c)
//
// These tests use evaluate_value() to verify routing behavior end-to-end
//
// Tests cover:
// - route.abort() actually blocks requests
// - route.continue() allows requests through
// - Error codes are properly applied
// - Request access in handlers
// - Conditional routing logic

mod test_server;

use playwright_rs::protocol::Playwright;
use test_server::TestServer;

#[tokio::test]
async fn test_route_abort_blocks_fetch() {
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

    // Set up route to abort image requests
    page.route("**/*.png", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Try to fetch an image - should fail due to abort
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

    assert_eq!(result, "failed", "Image request should have been aborted");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_continue_allows_fetch() {
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

    // Set up route that continues all requests
    page.route("**/*", |route| async move { route.continue_(None).await })
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
        "Request should succeed when continued"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_conditional_abort() {
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

    // Conditionally abort based on URL
    page.route("**/*", |route| async move {
        let request = route.request();
        if request.url().contains("block-me") {
            route.abort(None).await
        } else {
            route.continue_(None).await
        }
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Request with "block-me" should fail
    let blocked = page
        .evaluate_value(
            r#"
        fetch('/block-me')
            .then(() => 'success')
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");
    assert_eq!(
        blocked, "failed",
        "Request with 'block-me' should be blocked"
    );

    // Normal request should succeed
    let allowed = page
        .evaluate_value(
            r#"
        fetch('/allowed')
            .then(() => 'success')
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");
    assert_eq!(allowed, "success", "Normal request should succeed");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_pattern_specificity() {
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

    // Set up multiple routes with different patterns
    page.route("**/*.css", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up CSS route");

    page.route("**/*.js", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up JS route");

    page.route(
        "**/*.html",
        |route| async move { route.continue_(None).await },
    )
    .await
    .expect("Failed to set up HTML route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // CSS should be blocked
    let css_result = page
        .evaluate_value(
            r#"
        fetch('/style.css')
            .then(() => 'success')
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");
    assert_eq!(css_result, "failed", "CSS should be blocked");

    // JS should be blocked
    let js_result = page
        .evaluate_value(
            r#"
        fetch('/script.js')
            .then(() => 'success')
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");
    assert_eq!(js_result, "failed", "JS should be blocked");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// Cross-browser tests

#[tokio::test]
async fn test_route_abort_firefox() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");
    let page = browser.new_page().await.expect("Failed to create page");

    page.route("**/*.png", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

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

    assert_eq!(result, "failed", "Should work in Firefox");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_continue_webkit() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let page = browser.new_page().await.expect("Failed to create page");

    page.route("**/*", |route| async move { route.continue_(None).await })
        .await
        .expect("Failed to set up route");

    let response = page
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    assert_eq!(response.status(), 200, "Should work in WebKit");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
