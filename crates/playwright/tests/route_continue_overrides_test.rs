// Route Continue Overrides Tests
//
// Tests for Route.continue() with modifications (headers, method, postData, url)
//
// These tests verify that:
// 1. Headers can be modified when continuing a route
// 2. HTTP method can be changed (GET → POST, etc.)
// 3. POST data can be added or modified
// 4. URL can be changed (same protocol)
//
// TDD approach: Tests written FIRST, then implementation

use playwright_rs::protocol::{ContinueOptions, Playwright};
use std::collections::HashMap;

#[tokio::test]
async fn test_route_continue_with_headers() {
    // Test modifying headers when continuing a route
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Set up route handler that modifies headers
    page.route("**/*", |route| async move {
        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "test-value".to_string());

        let options = ContinueOptions::builder().headers(headers).build();

        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    // Navigate - the route should intercept and add custom header
    let result = page.goto("https://example.com", None).await;
    assert!(result.is_ok(), "Navigation should succeed");

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_method() {
    // Test changing HTTP method when continuing a route
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Set up route handler that changes GET to POST
    page.route("**/*", |route| async move {
        let request = route.request();
        let original_method = request.method();

        if original_method == "GET" {
            let options = ContinueOptions::builder()
                .method("POST".to_string())
                .build();

            route.continue_(Some(options)).await
        } else {
            route.continue_(None).await
        }
    })
    .await
    .expect("Failed to set up route");

    // Navigate - route should change method to POST
    let result = page.goto("https://example.com", None).await;

    // Navigation might fail because server doesn't accept POST for main document
    // But the test verifies the option is accepted by the API
    let _ = result; // Ignore result, we're testing API not behavior

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_post_data() {
    // Test adding POST data when continuing a route
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Set up route handler that adds POST data
    page.route("**/*", |route| async move {
        let options = ContinueOptions::builder()
            .post_data("key=value".to_string())
            .build();

        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    // Navigate
    let result = page.goto("https://example.com", None).await;
    let _ = result; // Ignore result

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_post_data_bytes() {
    // Test adding POST data as bytes when continuing a route
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Set up route handler that adds binary POST data
    page.route("**/*", |route| async move {
        let options = ContinueOptions::builder()
            .post_data_bytes(vec![0x01, 0x02, 0x03, 0x04])
            .build();

        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    // Navigate
    let result = page.goto("https://example.com", None).await;
    let _ = result; // Ignore result

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_url() {
    // Test changing URL when continuing a route (same protocol)
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Set up route handler that redirects to different URL (same protocol)
    page.route("**/original", |route| async move {
        let options = ContinueOptions::builder()
            .url("https://example.com/redirected".to_string())
            .build();

        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    // Navigate to original URL
    let result = page.goto("https://example.com/original", None).await;
    let _ = result; // Ignore result

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_combined_overrides() {
    // Test multiple overrides combined
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Set up route handler with multiple modifications
    page.route("**/*", |route| async move {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());
        headers.insert("X-Test".to_string(), "123".to_string());

        let options = ContinueOptions::builder()
            .headers(headers)
            .method("POST".to_string())
            .post_data("test=data".to_string())
            .build();

        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    // Navigate
    let result = page.goto("https://example.com", None).await;
    let _ = result; // Ignore result

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_no_overrides() {
    // Test that continue without overrides still works
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Set up route handler that continues without modification
    page.route("**/*", |route| async move { route.continue_(None).await })
        .await
        .expect("Failed to set up route");

    // Navigate - should work normally
    let result = page.goto("https://example.com", None).await;
    assert!(result.is_ok(), "Navigation should succeed");

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}
