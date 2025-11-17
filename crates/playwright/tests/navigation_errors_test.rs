// Integration tests for Navigation Error Handling (Phase 4, Slice 3)
//
// Following TDD: Write tests first (Red), then verify behavior (Green)
//
// Tests cover:
// - goto() timeout errors
// - reload() timeout errors
// - wait_until option behavior
// - Descriptive error messages
// - Cross-browser compatibility
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~67% (9 tests → 3 tests)

mod test_server;

use playwright_rs::protocol::{GotoOptions, Playwright, WaitUntil};
use std::time::Duration;
use test_server::TestServer;

// ============================================================================
// Navigation Error Methods
// ============================================================================

#[tokio::test]
async fn test_navigation_error_methods() {
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

    // Test 1: goto() timeout error with non-routable IP
    let options = GotoOptions::new().timeout(Duration::from_millis(100));
    let result = page.goto("http://10.255.255.1:9999/", Some(options)).await;

    assert!(result.is_err(), "Expected timeout error");

    // Error message should be descriptive
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("Timeout") || error_msg.contains("timeout"),
        "Error message should mention timeout: {}",
        error_msg
    );

    println!("✓ Timeout error with descriptive message");

    // Test 2: goto() with valid timeout should succeed
    let options = GotoOptions::new().timeout(Duration::from_secs(10));
    let result = page
        .goto(&format!("{}/locators.html", server.url()), Some(options))
        .await;

    assert!(
        result.is_ok(),
        "Navigation should succeed with valid timeout"
    );

    println!("✓ Navigation with valid timeout succeeds");

    // Test 3: goto() with invalid URL should error
    let result = page.goto("not-a-valid-url", None).await;
    assert!(result.is_err(), "Expected error for invalid URL");

    println!("✓ Invalid URL produces error");

    // Test 4: reload() with very short timeout
    // First navigate back to valid page
    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Navigation should succeed");

    let options = GotoOptions::new().timeout(Duration::from_millis(1));
    let result = page.reload(Some(options)).await;

    // May or may not timeout depending on timing, but should not crash
    let _ = result;

    println!("✓ Reload with short timeout handled gracefully");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Wait Until Options
// ============================================================================

#[tokio::test]
async fn test_wait_until_options() {
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

    // Test 1: wait_until Load
    let options = GotoOptions::new().wait_until(WaitUntil::Load);
    let result = page
        .goto(&format!("{}/locators.html", server.url()), Some(options))
        .await;

    assert!(
        result.is_ok(),
        "Navigation with wait_until=Load should succeed"
    );

    println!("✓ wait_until=Load works");

    // Test 2: wait_until DomContentLoaded
    let options = GotoOptions::new().wait_until(WaitUntil::DomContentLoaded);
    let result = page
        .goto(&format!("{}/locators.html", server.url()), Some(options))
        .await;

    assert!(
        result.is_ok(),
        "Navigation with wait_until=DOMContentLoaded should succeed"
    );

    println!("✓ wait_until=DomContentLoaded works");

    // Test 3: wait_until NetworkIdle
    let options = GotoOptions::new().wait_until(WaitUntil::NetworkIdle);
    let result = page
        .goto(&format!("{}/locators.html", server.url()), Some(options))
        .await;

    assert!(
        result.is_ok(),
        "Navigation with wait_until=NetworkIdle should succeed"
    );

    println!("✓ wait_until=NetworkIdle works");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    // Smoke test to verify navigation errors work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each method)

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test Firefox
    let firefox = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");
    let firefox_page = firefox.new_page().await.expect("Failed to create page");

    let options = GotoOptions::new().timeout(Duration::from_millis(100));
    let result = firefox_page
        .goto("http://10.255.255.1:9999/", Some(options))
        .await;

    assert!(result.is_err(), "Expected timeout error in Firefox");

    println!("✓ Firefox timeout error works");

    firefox.close().await.expect("Failed to close Firefox");

    // Test WebKit
    let webkit = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let webkit_page = webkit.new_page().await.expect("Failed to create page");

    let options = GotoOptions::new().timeout(Duration::from_millis(100));
    let result = webkit_page
        .goto("http://10.255.255.1:9999/", Some(options))
        .await;

    assert!(result.is_err(), "Expected timeout error in WebKit");

    println!("✓ WebKit timeout error works");

    webkit.close().await.expect("Failed to close WebKit");
}
