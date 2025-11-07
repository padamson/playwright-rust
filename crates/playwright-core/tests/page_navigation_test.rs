// Integration tests for Page navigation
//
// These tests verify that page navigation works correctly.
// Following TDD approach: Write tests first, then implement.
//
// TODO: Replace live internet URLs with local test server
// - Currently using example.com and rust-lang.org (fragile, requires network)
// - Should use Playwright's built-in test server or localhost HTTP server
// - Consider data: URLs for simple cases
// - See https://playwright.dev/docs/test-webserver for approach

use playwright_core::protocol::{GotoOptions, Playwright, WaitUntil};
use std::time::Duration;

#[tokio::test]
async fn test_page_goto_basic() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to example.com
    let response = page
        .goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Verify response
    assert!(response.ok(), "Response should be successful");
    assert_eq!(response.status(), 200, "Status should be 200");
    assert_eq!(response.url(), "https://example.com/");

    // Verify URL was updated
    assert_eq!(page.url(), "https://example.com/");

    println!("✓ Basic navigation successful");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_goto_with_options() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate with options
    let options = GotoOptions::new()
        .timeout(Duration::from_secs(60))
        .wait_until(WaitUntil::DomContentLoaded);

    let response = page
        .goto("https://example.com", Some(options))
        .await
        .expect("Failed to navigate with options");

    assert!(response.ok());
    assert_eq!(page.url(), "https://example.com/");

    println!("✓ Navigation with options successful");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_url_tracking() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Initially at about:blank
    assert_eq!(page.url(), "about:blank");

    // Navigate to first URL
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");
    assert_eq!(page.url(), "https://example.com/");

    // Navigate to second URL (rust-lang.org redirects to remove www)
    page.goto("https://www.rust-lang.org", None)
        .await
        .expect("Failed to navigate");
    assert_eq!(page.url(), "https://rust-lang.org/");

    println!("✓ URL tracking works correctly");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_title() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to example.com
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Get title
    let title = page.title().await.expect("Failed to get title");
    assert_eq!(title, "Example Domain");

    println!("✓ Page title: {}", title);

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_reload() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to example.com
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Reload page
    let response = page.reload(None).await.expect("Failed to reload page");

    assert!(response.ok());
    assert_eq!(response.status(), 200);
    assert_eq!(page.url(), "https://example.com/");

    println!("✓ Page reload successful");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_reload_with_options() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to example.com
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Reload with options
    let options = GotoOptions::new()
        .timeout(Duration::from_secs(60))
        .wait_until(WaitUntil::Load);

    let response = page
        .reload(Some(options))
        .await
        .expect("Failed to reload page");

    assert!(response.ok());
    assert_eq!(page.url(), "https://example.com/");

    println!("✓ Page reload with options successful");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

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

// Cross-browser tests - Verify navigation works on all three browser engines

#[tokio::test]
async fn test_firefox_navigation() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to example.com
    let response = page
        .goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Verify response
    assert!(response.ok(), "Response should be successful");
    assert_eq!(response.status(), 200, "Status should be 200");
    assert_eq!(response.url(), "https://example.com/");

    // Verify URL was updated
    assert_eq!(page.url(), "https://example.com/");

    // Verify title
    let title = page.title().await.expect("Failed to get title");
    assert_eq!(title, "Example Domain");

    println!("✓ Firefox navigation successful");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_webkit_navigation() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to example.com
    let response = page
        .goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Verify response
    assert!(response.ok(), "Response should be successful");
    assert_eq!(response.status(), 200, "Status should be 200");
    assert_eq!(response.url(), "https://example.com/");

    // Verify URL was updated
    assert_eq!(page.url(), "https://example.com/");

    // Verify title
    let title = page.title().await.expect("Failed to get title");
    assert_eq!(title, "Example Domain");

    println!("✓ WebKit navigation successful");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}
