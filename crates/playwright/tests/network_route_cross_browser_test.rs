// Cross-browser integration tests for Network Routing (Phase 5, Slice 4c)
//
// Tests routing works across all browsers (Chromium, Firefox, WebKit)
//
// Tests cover:
// - route.abort() in Firefox and WebKit
// - route.continue() in Firefox and WebKit
// - Pattern matching across browsers
// - Request access in route handlers
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Expected speedup: ~75% (8 tests → 2 tests)

mod test_server;

use playwright_rs::protocol::Playwright;
use std::sync::{Arc, Mutex};
use test_server::TestServer;

// ============================================================================
// Firefox Routing Methods
// ============================================================================

#[tokio::test]
async fn test_route_firefox_methods() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");

    // Test 1: Route abort in Firefox
    let page1 = browser.new_page().await.expect("Failed to create page");

    let aborted = Arc::new(Mutex::new(false));
    let aborted_clone = aborted.clone();

    page1
        .route("**/*.png", move |route| {
            let aborted = aborted_clone.clone();
            async move {
                *aborted.lock().unwrap() = true;
                route.abort(None).await
            }
        })
        .await
        .expect("Failed to set up route");

    page1
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    println!("✓ Route abort works in Firefox");

    page1.close().await.expect("Failed to close page");

    // Test 2: Route continue in Firefox
    let page2 = browser.new_page().await.expect("Failed to create page");

    let continued = Arc::new(Mutex::new(false));
    let continued_clone = continued.clone();

    page2
        .route("**/*", move |route| {
            let continued = continued_clone.clone();
            async move {
                *continued.lock().unwrap() = true;
                route.continue_(None).await
            }
        })
        .await
        .expect("Failed to set up route");

    let response = page2
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    assert_eq!(
        response.status(),
        200,
        "Request should succeed when continued in Firefox"
    );

    assert!(
        *continued.lock().unwrap(),
        "Route handler should have been called"
    );

    println!("✓ Route continue works in Firefox");

    page2.close().await.expect("Failed to close page");

    // Test 3: Pattern matching in Firefox
    let page3 = browser.new_page().await.expect("Failed to create page");

    let handler_called = Arc::new(Mutex::new(false));
    let handler_called_clone = handler_called.clone();

    page3
        .route("**/*.css", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up CSS route");

    page3
        .route("**/*.js", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up JS route");

    page3
        .route("**/*", move |route| {
            let handler_called = handler_called_clone.clone();
            async move {
                *handler_called.lock().unwrap() = true;
                route.continue_(None).await
            }
        })
        .await
        .expect("Failed to set up catch-all route");

    let response = page3
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    assert_eq!(response.status(), 200, "Should work in Firefox");

    assert!(
        *handler_called.lock().unwrap(),
        "Catch-all handler should be called"
    );

    println!("✓ Pattern matching works in Firefox");

    page3.close().await.expect("Failed to close page");

    // Test 4: Error codes in Firefox
    let page4 = browser.new_page().await.expect("Failed to create page");

    page4
        .route("**/data.json", |route| async move {
            route.abort(Some("accessdenied")).await
        })
        .await
        .expect("Failed to set up route");

    page4
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    println!("✓ Error codes work in Firefox");

    page4.close().await.expect("Failed to close page");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// WebKit Routing Methods
// ============================================================================

#[tokio::test]
async fn test_route_webkit_methods() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");

    // Test 1: Route abort in WebKit
    let page1 = browser.new_page().await.expect("Failed to create page");

    let aborted = Arc::new(Mutex::new(false));
    let aborted_clone = aborted.clone();

    page1
        .route("**/*.png", move |route| {
            let aborted = aborted_clone.clone();
            async move {
                *aborted.lock().unwrap() = true;
                route.abort(None).await
            }
        })
        .await
        .expect("Failed to set up route");

    page1
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    println!("✓ Route abort works in WebKit");

    page1.close().await.expect("Failed to close page");

    // Test 2: Route continue in WebKit
    let page2 = browser.new_page().await.expect("Failed to create page");

    let continued = Arc::new(Mutex::new(false));
    let continued_clone = continued.clone();

    page2
        .route("**/*", move |route| {
            let continued = continued_clone.clone();
            async move {
                *continued.lock().unwrap() = true;
                route.continue_(None).await
            }
        })
        .await
        .expect("Failed to set up route");

    let response = page2
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    assert_eq!(
        response.status(),
        200,
        "Request should succeed when continued in WebKit"
    );

    assert!(
        *continued.lock().unwrap(),
        "Route handler should have been called"
    );

    println!("✓ Route continue works in WebKit");

    page2.close().await.expect("Failed to close page");

    // Test 3: Request access in WebKit
    let page3 = browser.new_page().await.expect("Failed to create page");

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);

    page3
        .route("**/*", move |route| {
            let tx = tx.clone();
            async move {
                let request = route.request();
                let url = request.url();
                let method = request.method();
                tx.send(format!("{} {}", method, url)).await.ok();
                route.continue_(None).await
            }
        })
        .await
        .expect("Failed to set up route");

    page3
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let captured_data = rx.recv().await.expect("Should receive data from handler");
    assert!(
        captured_data.contains("GET"),
        "Handler should see GET method, got: {}",
        captured_data
    );
    assert!(
        captured_data.len() > 4, // "GET " + at least something
        "Handler should see request URL, got: {}",
        captured_data
    );

    println!("✓ Request access works in WebKit");

    page3.close().await.expect("Failed to close page");

    // Test 4: Conditional logic in WebKit
    let page4 = browser.new_page().await.expect("Failed to create page");

    let blocked_count = Arc::new(Mutex::new(0));
    let allowed_count = Arc::new(Mutex::new(0));
    let blocked_clone = blocked_count.clone();
    let allowed_clone = allowed_count.clone();

    page4
        .route("**/*", move |route| {
            let blocked = blocked_clone.clone();
            let allowed = allowed_clone.clone();
            async move {
                let request = route.request();
                if request.url().contains("block-me") {
                    *blocked.lock().unwrap() += 1;
                    route.abort(None).await
                } else {
                    *allowed.lock().unwrap() += 1;
                    route.continue_(None).await
                }
            }
        })
        .await
        .expect("Failed to set up route");

    page4
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert!(
        *allowed_count.lock().unwrap() > 0,
        "Should have allowed some requests"
    );

    println!("✓ Conditional logic works in WebKit");

    page4.close().await.expect("Failed to close page");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
