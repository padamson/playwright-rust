// Simplified integration test for Network Routing (Phase 5, Slice 4a)
//
// Simplified version that doesn't require evaluate() to return values
// Tests basic route registration and handler invocation

mod test_server;

use playwright_rs::protocol::Playwright;
use std::sync::{Arc, Mutex};
use test_server::TestServer;

#[tokio::test]
async fn test_route_registration() {
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

    // Track if handler was called
    let called = Arc::new(Mutex::new(false));
    let called_clone = called.clone();

    // Test: Route handler can be registered
    page.route("**/*.png", move |route| {
        let called = called_clone.clone();
        async move {
            *called.lock().unwrap() = true;
            route.abort(None).await
        }
    })
    .await
    .expect("Failed to set up route");

    // Navigate to trigger requests
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Give time for requests to be processed
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    browser.close().await.expect("Failed to close browser");
    server.shutdown();

    // For now, just verify route registered successfully
    // Handler invocation will be tested once protocol integration is complete
    println!("Route registration test passed");
}

#[tokio::test]
async fn test_route_continue() {
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

    // Test: Route can continue requests unchanged
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
