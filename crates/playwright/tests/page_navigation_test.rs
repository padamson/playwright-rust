// Integration tests for Page navigation
//
// These tests verify that page navigation works correctly.
// Following TDD approach: Write tests first, then implement.
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~67% (9 tests → 3 tests)
//
// TODO: Refactor to use test_server.rs instead of external URLs
// Currently using example.com and rust-lang.org (fragile, requires network)
// Should use local test server with custom HTML for deterministic testing
// See locator_test.rs for refactored example using test_server

use playwright_rs::protocol::{GotoOptions, Playwright, WaitUntil};
use std::time::Duration;

// ============================================================================
// Page Navigation Methods
// ============================================================================

#[tokio::test]
async fn test_page_navigation_methods() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Test 1: Basic goto navigation
    let response = page
        .goto("https://example.com", None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response from https://example.com");

    assert!(response.ok(), "Response should be successful");
    assert_eq!(response.status(), 200, "Status should be 200");
    assert_eq!(response.url(), "https://example.com/");
    assert_eq!(page.url(), "https://example.com/");

    println!("✓ Basic navigation successful");

    // Test 2: Get page title
    let title = page.title().await.expect("Failed to get title");
    assert_eq!(title, "Example Domain");
    println!("✓ Page title: {}", title);

    // Test 3: URL tracking across navigations
    // Navigate to second URL (rust-lang.org redirects to remove www)
    page.goto("https://www.rust-lang.org", None)
        .await
        .expect("Failed to navigate");
    assert_eq!(page.url(), "https://rust-lang.org/");
    println!("✓ URL tracking works correctly");

    // Test 4: Navigate with options
    let options = GotoOptions::new()
        .timeout(Duration::from_secs(60))
        .wait_until(WaitUntil::DomContentLoaded);

    let response = page
        .goto("https://example.com", Some(options))
        .await
        .expect("Failed to navigate with options")
        .expect("Expected a response from https://example.com");

    assert!(response.ok());
    assert_eq!(page.url(), "https://example.com/");
    println!("✓ Navigation with options successful");

    // Test 5: Page reload
    let response = page
        .reload(None)
        .await
        .expect("Failed to reload page")
        .expect("Expected a response from https://example.com");

    assert!(response.ok());
    assert_eq!(response.status(), 200);
    assert_eq!(page.url(), "https://example.com/");
    println!("✓ Page reload successful");

    // Test 6: Reload with options
    let options = GotoOptions::new()
        .timeout(Duration::from_secs(60))
        .wait_until(WaitUntil::Load);

    let response = page
        .reload(Some(options))
        .await
        .expect("Failed to reload page")
        .expect("Expected a response from https://example.com");

    assert!(response.ok());
    assert_eq!(page.url(), "https://example.com/");
    println!("✓ Page reload with options successful");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Multiple Pages
// ============================================================================

#[tokio::test]
async fn test_multiple_pages_independent_urls() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create two pages
    let page1 = browser.new_page().await.expect("Failed to create page 1");
    let page2 = browser.new_page().await.expect("Failed to create page 2");

    // Initially both at about:blank
    assert_eq!(page1.url(), "about:blank");
    assert_eq!(page2.url(), "about:blank");

    // Navigate to different URLs
    page1
        .goto("https://example.com", None)
        .await
        .expect("Failed to navigate page 1");
    page2
        .goto("https://www.rust-lang.org", None)
        .await
        .expect("Failed to navigate page 2");

    // Verify URLs are independent (rust-lang.org redirects to remove www)
    assert_eq!(page1.url(), "https://example.com/");
    assert_eq!(page2.url(), "https://rust-lang.org/");

    println!("✓ Multiple pages have independent URLs");

    // Cleanup
    page1.close().await.expect("Failed to close page 1");
    page2.close().await.expect("Failed to close page 2");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    // Smoke test to verify navigation works in Firefox and WebKit
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

    let response = firefox_page
        .goto("https://example.com", None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response from https://example.com");

    assert!(response.ok(), "Response should be successful");
    assert_eq!(response.status(), 200, "Status should be 200");
    assert_eq!(response.url(), "https://example.com/");
    assert_eq!(firefox_page.url(), "https://example.com/");

    let title = firefox_page.title().await.expect("Failed to get title");
    assert_eq!(title, "Example Domain");

    println!("✓ Firefox navigation successful");

    firefox_page
        .close()
        .await
        .expect("Failed to close Firefox page");
    firefox.close().await.expect("Failed to close Firefox");

    // Test WebKit
    let webkit = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");

    let webkit_page = webkit.new_page().await.expect("Failed to create page");

    let response = webkit_page
        .goto("https://example.com", None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response from https://example.com");

    assert!(response.ok(), "Response should be successful");
    assert_eq!(response.status(), 200, "Status should be 200");
    assert_eq!(response.url(), "https://example.com/");
    assert_eq!(webkit_page.url(), "https://example.com/");

    let title = webkit_page.title().await.expect("Failed to get title");
    assert_eq!(title, "Example Domain");

    println!("✓ WebKit navigation successful");

    webkit_page
        .close()
        .await
        .expect("Failed to close WebKit page");
    webkit.close().await.expect("Failed to close WebKit");
}
