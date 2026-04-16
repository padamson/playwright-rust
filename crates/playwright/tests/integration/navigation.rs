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

use crate::test_server::TestServer;
use playwright_rs::protocol::{GotoOptions, Playwright, WaitUntil};
use std::time::Duration;

// ============================================================================
// Page Navigation Methods
// ============================================================================

#[tokio::test]
async fn test_page_navigation_methods() {
    crate::common::init_tracing();
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

    tracing::info!("✓ Basic navigation successful");

    // Test 2: Get page title
    let title = page.title().await.expect("Failed to get title");
    assert_eq!(title, "Example Domain");
    tracing::info!("✓ Page title: {}", title);

    // Test 3: URL tracking across navigations
    // Navigate to second URL (rust-lang.org redirects to remove www)
    page.goto("https://www.rust-lang.org", None)
        .await
        .expect("Failed to navigate");
    assert_eq!(page.url(), "https://rust-lang.org/");
    tracing::info!("✓ URL tracking works correctly");

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
    tracing::info!("✓ Navigation with options successful");

    // Test 5: Page reload
    let response = page
        .reload(None)
        .await
        .expect("Failed to reload page")
        .expect("Expected a response from https://example.com");

    assert!(response.ok());
    assert_eq!(response.status(), 200);
    assert_eq!(page.url(), "https://example.com/");
    tracing::info!("✓ Page reload successful");

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
    tracing::info!("✓ Page reload with options successful");

    // Test 7: Navigate to second page, then go_back
    page.goto("https://www.rust-lang.org", None)
        .await
        .expect("Failed to navigate to rust-lang.org");
    assert_eq!(page.url(), "https://rust-lang.org/");

    let response = page
        .go_back(None)
        .await
        .expect("Failed to go back")
        .expect("Expected a response when going back");
    assert_eq!(response.status(), 200);
    assert_eq!(page.url(), "https://example.com/");
    tracing::info!("✓ go_back() navigated to previous page");

    // Test 8: go_forward to return
    let response = page
        .go_forward(None)
        .await
        .expect("Failed to go forward")
        .expect("Expected a response when going forward");
    assert_eq!(response.status(), 200);
    assert_eq!(page.url(), "https://rust-lang.org/");
    tracing::info!("✓ go_forward() navigated to next page");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Multiple Pages
// ============================================================================

#[tokio::test]
async fn test_multiple_pages_independent_urls() {
    crate::common::init_tracing();
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

    tracing::info!("✓ Multiple pages have independent URLs");

    // Cleanup
    page1.close().await.expect("Failed to close page 1");
    page2.close().await.expect("Failed to close page 2");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_cross_browser_smoke() {
    crate::common::init_tracing();
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

    tracing::info!("✓ Firefox navigation successful");

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

    tracing::info!("✓ WebKit navigation successful");

    webkit_page
        .close()
        .await
        .expect("Failed to close WebKit page");
    webkit.close().await.expect("Failed to close WebKit");
}

// ============================================================================
// Merged from: navigation_errors_test.rs
// ============================================================================

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

// ============================================================================
// Navigation Error Methods
// ============================================================================

#[tokio::test]
async fn test_navigation_error_methods() {
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

    tracing::info!("✓ Timeout error with descriptive message");

    // Test 2: goto() with valid timeout should succeed
    let options = GotoOptions::new().timeout(Duration::from_secs(10));
    let result = page
        .goto(&format!("{}/locators.html", server.url()), Some(options))
        .await;

    assert!(
        result.is_ok(),
        "Navigation should succeed with valid timeout"
    );

    tracing::info!("✓ Navigation with valid timeout succeeds");

    // Test 3: goto() with invalid URL should error
    let result = page.goto("not-a-valid-url", None).await;
    assert!(result.is_err(), "Expected error for invalid URL");

    tracing::info!("✓ Invalid URL produces error");

    // Test 4: reload() with very short timeout
    // First navigate back to valid page
    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Navigation should succeed");

    let options = GotoOptions::new().timeout(Duration::from_millis(1));
    let result = page.reload(Some(options)).await;

    // May or may not timeout depending on timing, but should not crash
    let _ = result;

    tracing::info!("✓ Reload with short timeout handled gracefully");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Wait Until Options
// ============================================================================

#[tokio::test]
async fn test_wait_until_options() {
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

    // Test 1: wait_until Load
    let options = GotoOptions::new().wait_until(WaitUntil::Load);
    let result = page
        .goto(&format!("{}/locators.html", server.url()), Some(options))
        .await;

    assert!(
        result.is_ok(),
        "Navigation with wait_until=Load should succeed"
    );

    tracing::info!("✓ wait_until=Load works");

    // Test 2: wait_until DomContentLoaded
    let options = GotoOptions::new().wait_until(WaitUntil::DomContentLoaded);
    let result = page
        .goto(&format!("{}/locators.html", server.url()), Some(options))
        .await;

    assert!(
        result.is_ok(),
        "Navigation with wait_until=DOMContentLoaded should succeed"
    );

    tracing::info!("✓ wait_until=DomContentLoaded works");

    // Test 3: wait_until NetworkIdle
    let options = GotoOptions::new().wait_until(WaitUntil::NetworkIdle);
    let result = page
        .goto(&format!("{}/locators.html", server.url()), Some(options))
        .await;

    assert!(
        result.is_ok(),
        "Navigation with wait_until=NetworkIdle should succeed"
    );

    tracing::info!("✓ wait_until=NetworkIdle works");

    // Test 4: wait_for_load_state with Load
    page.wait_for_load_state(Some(WaitUntil::Load))
        .await
        .expect("wait_for_load_state(Load) should succeed");
    tracing::info!("✓ wait_for_load_state(Load) works");

    // Test 5: wait_for_load_state with DomContentLoaded
    page.wait_for_load_state(Some(WaitUntil::DomContentLoaded))
        .await
        .expect("wait_for_load_state(DomContentLoaded) should succeed");
    tracing::info!("✓ wait_for_load_state(DomContentLoaded) works");

    // Test 6: wait_for_load_state with None (defaults to "load")
    page.wait_for_load_state(None)
        .await
        .expect("wait_for_load_state(None) should succeed");
    tracing::info!("✓ wait_for_load_state(None) works");

    // Test 7: wait_for_url with exact match (already at the URL)
    let current_url = format!("{}/locators.html", server.url());
    page.wait_for_url(&current_url, None)
        .await
        .expect("wait_for_url should succeed for current URL");
    tracing::info!("✓ wait_for_url(exact) works");

    // Test 8: wait_for_url with glob pattern
    page.wait_for_url(&format!("{}/**", server.url()), None)
        .await
        .expect("wait_for_url with glob should succeed");
    tracing::info!("✓ wait_for_url(glob) works");

    // Test 9: wait_for_url timeout for non-matching URL
    let options = GotoOptions::new().timeout(Duration::from_millis(100));
    let result = page
        .wait_for_url("http://never-matches.example.com/", Some(options))
        .await;
    assert!(
        result.is_err(),
        "Expected timeout error for non-matching URL"
    );
    tracing::info!("✓ wait_for_url timeout works");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_navigation_errors_cross_browser_smoke() {
    crate::common::init_tracing();
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

    tracing::info!("✓ Firefox timeout error works");

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

    tracing::info!("✓ WebKit timeout error works");

    webkit.close().await.expect("Failed to close WebKit");
}

// ============================================================================
// Merged from: page_url_hash_navigation_test.rs
// ============================================================================

// Test for page.url() hash navigation behavior (Issue #26)
//
// Verifies that page.url() correctly reflects URL changes when navigating
// via anchor links (hash fragments).

/// Test that page.url() returns URL with hash after anchor navigation
#[tokio::test]
async fn test_url_includes_hash_after_anchor_click() {
    crate::common::init_tracing();

    let server = TestServer::start().await;
    let base_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to test page with anchors
    let url = format!("{}/anchors.html", base_url);
    page.goto(&url, None).await.expect("Failed to navigate");

    // Verify initial URL (without hash)
    assert_eq!(page.url(), url);
    tracing::info!("Initial URL: {}", page.url());

    // Click anchor link to navigate to #section1
    let anchor = page.locator("#link-to-section1").await;
    anchor.click(None).await.expect("Failed to click anchor");

    // Wait a bit for navigation to settle
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Verify URL now includes the hash
    let expected_url = format!("{}#section1", url);
    let actual_url = page.url();
    tracing::info!("URL after anchor click: {}", actual_url);

    assert_eq!(
        actual_url, expected_url,
        "Expected URL '{}' but got '{}'",
        expected_url, actual_url
    );

    // Click another anchor to navigate to #section2
    let anchor2 = page.locator("#link-to-section2").await;
    anchor2
        .click(None)
        .await
        .expect("Failed to click second anchor");

    // Wait a bit for navigation to settle
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Verify URL updated to new hash
    let expected_url2 = format!("{}#section2", url);
    let actual_url2 = page.url();
    tracing::info!("URL after second anchor click: {}", actual_url2);

    assert_eq!(
        actual_url2, expected_url2,
        "Expected URL '{}' but got '{}'",
        expected_url2, actual_url2
    );

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that page.url() reflects JavaScript location.hash changes
#[tokio::test]
async fn test_url_includes_hash_after_js_navigation() {
    crate::common::init_tracing();

    let server = TestServer::start().await;
    let base_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to test page
    let url = format!("{}/anchors.html", base_url);
    page.goto(&url, None).await.expect("Failed to navigate");

    // Verify initial URL (without hash)
    assert_eq!(page.url(), url);

    // Use JavaScript to change the hash
    page.evaluate_expression("window.location.hash = '#js-section'")
        .await
        .expect("Failed to execute JavaScript");

    // Wait a bit for navigation to settle
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Verify URL includes the hash set via JavaScript
    let expected_url = format!("{}#js-section", url);
    let actual_url = page.url();
    tracing::info!("URL after JS hash change: {}", actual_url);

    assert_eq!(
        actual_url, expected_url,
        "Expected URL '{}' but got '{}'",
        expected_url, actual_url
    );

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test cross-browser: Hash navigation works on Chromium, Firefox, and WebKit
#[tokio::test]
#[ignore]
async fn test_url_hash_cross_browser() {
    crate::common::init_tracing();

    let server = TestServer::start().await;
    let base_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test on all three browser engines
    for browser_name in ["chromium", "firefox", "webkit"] {
        tracing::info!("Testing hash navigation on {}", browser_name);

        let browser = match browser_name {
            "chromium" => playwright.chromium().launch().await,
            "firefox" => playwright.firefox().launch().await,
            "webkit" => playwright.webkit().launch().await,
            _ => unreachable!(),
        }
        .unwrap_or_else(|_| panic!("Failed to launch {}", browser_name));

        let page = browser.new_page().await.expect("Failed to create page");

        // Navigate to test page
        let url = format!("{}/anchors.html", base_url);
        page.goto(&url, None).await.expect("Failed to navigate");

        // Click anchor link
        let anchor = page.locator("#link-to-section1").await;
        anchor.click(None).await.expect("Failed to click anchor");

        // Wait for navigation
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify URL includes hash on this browser
        let expected_url = format!("{}#section1", url);
        let actual_url = page.url();

        assert_eq!(
            actual_url, expected_url,
            "Hash navigation failed on {}: expected '{}' but got '{}'",
            browser_name, expected_url, actual_url
        );

        tracing::info!("✓ {} hash navigation works", browser_name);

        // Cleanup
        page.close().await.expect("Failed to close page");
        browser.close().await.expect("Failed to close browser");
    }

    server.shutdown();
}

// ============================================================================
// History Navigation Edge Cases
// ============================================================================

#[tokio::test]
async fn test_page_go_back_no_history() {
    crate::common::init_tracing();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // go_back on fresh page should return None
    let result = page.go_back(None).await.expect("go_back should not error");
    assert!(result.is_none(), "Expected None when no history to go back");
    tracing::info!("✓ go_back() returns None on fresh page");

    // go_forward on fresh page should return None
    let result = page
        .go_forward(None)
        .await
        .expect("go_forward should not error");
    assert!(
        result.is_none(),
        "Expected None when no history to go forward"
    );
    tracing::info!("✓ go_forward() returns None on fresh page");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

/// Test that workaround using evaluate_value still works
#[tokio::test]
async fn test_url_workaround_with_evaluate() {
    crate::common::init_tracing();

    let server = TestServer::start().await;
    let base_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to test page
    let url = format!("{}/anchors.html", base_url);
    page.goto(&url, None).await.expect("Failed to navigate");

    // Click anchor link
    let anchor = page.locator("#link-to-section1").await;
    anchor.click(None).await.expect("Failed to click anchor");

    // Wait for navigation
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Workaround: Use evaluate_value to get current URL
    let url_via_evaluate = page
        .evaluate_value("window.location.href")
        .await
        .expect("Failed to evaluate window.location.href");

    let expected_url = format!("{}#section1", url);

    // The workaround should always work
    assert_eq!(
        url_via_evaluate, expected_url,
        "Workaround failed: expected '{}' but got '{}'",
        expected_url, url_via_evaluate
    );

    tracing::info!("✓ Workaround using evaluate_value works");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
