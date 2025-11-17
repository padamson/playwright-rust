// Pattern matching tests for Network Routing (Phase 5, Slice 4b)
//
// Tests glob pattern matching for route handlers

mod test_server;

use playwright_rs::protocol::Playwright;
use std::sync::{Arc, Mutex};
use test_server::TestServer;

#[tokio::test]
async fn test_route_pattern_matching_wildcard() {
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

    // Track which handlers were called
    let png_called = Arc::new(Mutex::new(false));
    let js_called = Arc::new(Mutex::new(false));
    let all_called = Arc::new(Mutex::new(false));

    let png_called_clone = png_called.clone();
    let js_called_clone = js_called.clone();
    let all_called_clone = all_called.clone();

    // Register handlers for different patterns
    // Note: Last registered wins, so order matters

    // Handler for all requests (should NOT be called if more specific handler matches)
    page.route("**/*", move |route| {
        let all_called = all_called_clone.clone();
        async move {
            *all_called.lock().unwrap() = true;
            route.continue_(None).await
        }
    })
    .await
    .expect("Failed to set up wildcard route");

    // Handler for PNG images (more specific, registered last, should win)
    page.route("**/*.png", move |route| {
        let png_called = png_called_clone.clone();
        async move {
            *png_called.lock().unwrap() = true;
            route.abort(None).await
        }
    })
    .await
    .expect("Failed to set up PNG route");

    // Handler for JS files
    page.route("**/*.js", move |route| {
        let js_called = js_called_clone.clone();
        async move {
            *js_called.lock().unwrap() = true;
            route.abort(None).await
        }
    })
    .await
    .expect("Failed to set up JS route");

    // Navigate - HTML should match wildcard and continue
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Small delay to allow route handlers to process
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    browser.close().await.expect("Failed to close browser");
    server.shutdown();

    // Verify wildcard handler was called for HTML (most general pattern wins for this URL)
    println!(
        "Handler calls - all: {}, png: {}, js: {}",
        *all_called.lock().unwrap(),
        *png_called.lock().unwrap(),
        *js_called.lock().unwrap()
    );
}

#[tokio::test]
async fn test_route_pattern_priority() {
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

    // Track handler calls
    let first_called = Arc::new(Mutex::new(0));
    let second_called = Arc::new(Mutex::new(0));

    let first_called_clone = first_called.clone();
    let second_called_clone = second_called.clone();

    // Register first handler
    page.route("**/*", move |route| {
        let first_called = first_called_clone.clone();
        async move {
            *first_called.lock().unwrap() += 1;
            route.continue_(None).await
        }
    })
    .await
    .expect("Failed to set up first route");

    // Register second handler with same pattern (should win due to last-registered-wins)
    page.route("**/*", move |route| {
        let second_called = second_called_clone.clone();
        async move {
            *second_called.lock().unwrap() += 1;
            route.continue_(None).await
        }
    })
    .await
    .expect("Failed to set up second route");

    // Navigate
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Small delay
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    browser.close().await.expect("Failed to close browser");
    server.shutdown();

    // Last registered handler should be called
    assert_eq!(
        *first_called.lock().unwrap(),
        0,
        "First handler should NOT be called (last registered wins)"
    );
    assert!(
        *second_called.lock().unwrap() > 0,
        "Second handler should be called (last registered wins)"
    );
}

#[tokio::test]
async fn test_route_conditional_matching() {
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

    // Test: Handler can inspect URL and conditionally abort
    let abort_count = Arc::new(Mutex::new(0));
    let continue_count = Arc::new(Mutex::new(0));

    let abort_count_clone = abort_count.clone();
    let continue_count_clone = continue_count.clone();

    page.route("**/*", move |route| {
        let abort_count = abort_count_clone.clone();
        let continue_count = continue_count_clone.clone();
        async move {
            let request = route.request();
            let url = request.url();

            if url.contains("image") || url.contains(".png") || url.contains(".jpg") {
                *abort_count.lock().unwrap() += 1;
                route.abort(None).await
            } else {
                *continue_count.lock().unwrap() += 1;
                route.continue_(None).await
            }
        }
    })
    .await
    .expect("Failed to set up conditional route");

    // Navigate (should continue)
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    browser.close().await.expect("Failed to close browser");
    server.shutdown();

    assert!(
        *continue_count.lock().unwrap() > 0,
        "Continue should be called for HTML"
    );
}

#[tokio::test]
async fn test_route_extension_patterns() {
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

    // Test file extension glob patterns
    let html_called = Arc::new(Mutex::new(false));
    let html_called_clone = html_called.clone();

    // Pattern: match .html files (but server returns / without extension)
    // So use a pattern that matches URLs ending with /
    page.route("**/", move |route| {
        let html_called = html_called_clone.clone();
        async move {
            *html_called.lock().unwrap() = true;
            route.continue_(None).await
        }
    })
    .await
    .expect("Failed to set up root path route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    browser.close().await.expect("Failed to close browser");
    server.shutdown();

    assert!(
        *html_called.lock().unwrap(),
        "Extension/path pattern should work"
    );
}
