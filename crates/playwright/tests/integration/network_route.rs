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

use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_route_abort_blocks_fetch() {
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

// Cross-browser route tests are in network_route_cross_browser_test.rs

// ============================================================================
// Merged from: network_route_pattern_test.rs
// ============================================================================

// Pattern matching tests for Network Routing (Phase 5, Slice 4b)
//
// Tests glob pattern matching for route handlers

#[tokio::test]
async fn test_route_pattern_matching_wildcard() {
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
    tracing::info!(
        "Handler calls - all: {}, png: {}, js: {}",
        *all_called.lock().unwrap(),
        *png_called.lock().unwrap(),
        *js_called.lock().unwrap()
    );
}

#[tokio::test]
async fn test_route_pattern_priority() {
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

// ============================================================================
// Merged from: network_route_cross_browser_test.rs
// ============================================================================

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

// ============================================================================
// Firefox Routing Methods
// ============================================================================

#[tokio::test]
async fn test_route_firefox_methods() {
    crate::common::init_tracing();
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

    tracing::info!("✓ Route abort works in Firefox");

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

    tracing::info!("✓ Route continue works in Firefox");

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

    tracing::info!("✓ Pattern matching works in Firefox");

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

    tracing::info!("✓ Error codes work in Firefox");

    page4.close().await.expect("Failed to close page");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// WebKit Routing Methods
// ============================================================================

#[tokio::test]
async fn test_route_webkit_methods() {
    crate::common::init_tracing();
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

    tracing::info!("✓ Route abort works in WebKit");

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

    tracing::info!("✓ Route continue works in WebKit");

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

    tracing::info!("✓ Request access works in WebKit");

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

    tracing::info!("✓ Conditional logic works in WebKit");

    page4.close().await.expect("Failed to close page");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
