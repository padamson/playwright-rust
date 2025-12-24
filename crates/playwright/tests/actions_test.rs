// Integration tests for Locator actions
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - Click actions (single, double)
// - Fill actions (input, textarea)
// - Clear actions
// - Press actions (keyboard)
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~50% (8 tests → 4 tests)

mod test_server;

use playwright_rs::protocol::{GotoOptions, Playwright};
use test_server::TestServer;

// ============================================================================
// Click Actions
// ============================================================================

#[tokio::test]
async fn test_click_actions() {
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

    // Test 1: Single click button changes its text
    page.goto(&format!("{}/button.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let button = page.locator("#btn").await;
    button.click(None).await.expect("Failed to click button");

    let text = button.text_content().await.expect("Failed to get text");
    assert_eq!(text, Some("clicked".to_string()));

    // Test 2: Double-click changes div text
    page.goto(&format!("{}/dblclick.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let div = page.locator("#target").await;
    div.dblclick(None).await.expect("Failed to double-click");

    let text = div.text_content().await.expect("Failed to get text");
    assert_eq!(text, Some("double clicked".to_string()));

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Fill and Clear Actions
// ============================================================================

#[tokio::test]
async fn test_fill_and_clear_actions() {
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

    page.goto(&format!("{}/form.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Fill input field
    let input = page.locator("#name").await;
    input
        .fill("John Doe", None)
        .await
        .expect("Failed to fill input");

    let value = input
        .input_value(None)
        .await
        .expect("Failed to get input value");
    assert_eq!(value, "John Doe");

    // Test 2: Fill textarea
    let textarea = page.locator("#bio").await;
    textarea
        .fill("Hello\nWorld", None)
        .await
        .expect("Failed to fill textarea");

    let value = textarea
        .input_value(None)
        .await
        .expect("Failed to get textarea value");
    assert_eq!(value, "Hello\nWorld");

    // Test 3: Clear input field with initial value
    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let clear_input = page.locator("#input").await;

    // Verify initial value
    let initial_value = clear_input
        .input_value(None)
        .await
        .expect("Failed to get initial value");
    assert_eq!(initial_value, "initial");

    // Clear the input
    clear_input
        .clear(None)
        .await
        .expect("Failed to clear input");

    // Verify input is now empty
    let cleared_value = clear_input
        .input_value(None)
        .await
        .expect("Failed to get cleared value");
    assert_eq!(cleared_value, "");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Keyboard Actions
// ============================================================================

#[tokio::test]
async fn test_keyboard_actions() {
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

    page.goto(&format!("{}/keyboard.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test: Press Enter key changes input value via JavaScript
    let input = page.locator("#input").await;
    input.click(None).await.expect("Failed to focus input");
    input
        .press("Enter", None)
        .await
        .expect("Failed to press Enter");

    // Verify keypress had effect (keyboard.html sets value to "submitted" on Enter)
    let value = input
        .input_value(None)
        .await
        .expect("Failed to get input value");
    assert_eq!(value, "submitted");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    // Smoke test to verify actions work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each action)

    let server = TestServer::start().await;
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

    firefox_page
        .goto(
            &format!("{}/button.html", server.url()),
            Some(GotoOptions::new().timeout(std::time::Duration::from_secs(60))),
        )
        .await
        .expect("Failed to navigate");

    let firefox_button = firefox_page.locator("#btn").await;
    firefox_button
        .click(None)
        .await
        .expect("Failed to click button");

    let text = firefox_button
        .text_content()
        .await
        .expect("Failed to get text");
    assert_eq!(text, Some("clicked".to_string()));

    firefox.close().await.expect("Failed to close Firefox");

    // Test WebKit
    let webkit = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let webkit_page = webkit.new_page().await.expect("Failed to create page");

    webkit_page
        .goto(
            &format!("{}/form.html", server.url()),
            Some(GotoOptions::new().timeout(std::time::Duration::from_secs(60))),
        )
        .await
        .expect("Failed to navigate");

    let webkit_input = webkit_page.locator("#name").await;
    webkit_input
        .fill("Test", None)
        .await
        .expect("Failed to fill input");

    // Verify the input value
    let value = webkit_input
        .input_value(None)
        .await
        .expect("Failed to get input value");
    assert_eq!(value, "Test");

    webkit.close().await.expect("Failed to close WebKit");
    server.shutdown();
}
