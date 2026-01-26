// Integration tests for Click Options (Phase 4, Slice 4)
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - Click with button option (left, right, middle)
// - Click with modifiers (Shift, Control, etc.)
// - Click with position option
// - Click with force option
// - Click with trial option (dry-run)
// - Double-click with click_count
// - Cross-browser compatibility
//
// Note: Tests are combined where possible to reduce browser launches

mod test_server;

use playwright_rs::protocol::Playwright;
use playwright_rs::protocol::click::{ClickOptions, KeyboardModifier, MouseButton, Position};
use test_server::TestServer;

mod common;

#[tokio::test]
async fn test_click_with_button_options() {
    common::init_tracing();
    // Combined test: All button options in one browser session
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

    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Default click (left button)
    let button = page.locator("#button").await;
    button.click(None).await.expect("Failed to click");

    let result = page.locator("#result").await.inner_text().await.unwrap();
    assert!(result.contains("left"), "Default should be left click");

    // Reset
    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 2: Right click
    let options = ClickOptions::builder().button(MouseButton::Right).build();
    button
        .click(Some(options))
        .await
        .expect("Failed to right-click");

    let result = page.locator("#result").await.inner_text().await.unwrap();
    // Right click can trigger contextmenu or auxclick event
    assert!(
        result.contains("contextmenu") || result.contains("right") || result.contains("auxclick"),
        "Should register right click: {}",
        result
    );

    // Reset
    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 3: Middle click
    let options = ClickOptions::builder().button(MouseButton::Middle).build();
    button
        .click(Some(options))
        .await
        .expect("Failed to middle-click");

    let result = page.locator("#result").await.inner_text().await.unwrap();
    assert!(
        result.contains("middle"),
        "Should register middle click: {}",
        result
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_click_with_modifiers() {
    common::init_tracing();
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

    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Click with Shift modifier
    let button = page.locator("#button").await;
    let options = ClickOptions::builder()
        .modifiers(vec![KeyboardModifier::Shift])
        .build();
    button
        .click(Some(options))
        .await
        .expect("Failed to click with modifiers");

    let result = page.locator("#result").await.inner_text().await.unwrap();
    assert!(
        result.contains("shiftKey:true"),
        "Should have Shift modifier: {}",
        result
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_click_with_position() {
    common::init_tracing();
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

    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Click at specific position
    let button = page.locator("#button").await;
    let options = ClickOptions::builder()
        .position(Position { x: 10.0, y: 10.0 })
        .build();
    button
        .click(Some(options))
        .await
        .expect("Failed to click with position");

    // Just verify click worked (position is relative to element)
    let result = page.locator("#result").await.inner_text().await.unwrap();
    assert!(
        !result.is_empty(),
        "Click with position should trigger event"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_click_with_force() {
    common::init_tracing();
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

    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Click button with force option (verifies option is passed correctly)
    let button = page.locator("#button").await;
    let options = ClickOptions::builder().force(true).build();

    let result = button.click(Some(options)).await;

    // Force click should succeed
    assert!(result.is_ok(), "Force click should succeed");

    // Verify click was registered
    let text = page.locator("#result").await.inner_text().await.unwrap();
    assert!(!text.is_empty(), "Click should have been registered");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_click_with_trial() {
    common::init_tracing();
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

    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Trial click (should not actually click)
    let button = page.locator("#button").await;
    let options = ClickOptions::builder().trial(true).build();
    button
        .click(Some(options))
        .await
        .expect("Failed to trial click");

    // Result should still be empty since trial doesn't actually click
    let result = page.locator("#result").await.inner_text().await.unwrap();
    assert!(
        result.is_empty(),
        "Trial click should not trigger event: {}",
        result
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_dblclick_with_options() {
    common::init_tracing();
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

    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Double-click
    let button = page.locator("#button").await;
    button.dblclick(None).await.expect("Failed to double-click");

    let result = page.locator("#result").await.inner_text().await.unwrap();
    assert!(
        result.contains("dblclick"),
        "Should register double-click: {}",
        result
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_click_options_firefox() {
    common::init_tracing();
    // Cross-browser test: Firefox
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let button = page.locator("#button").await;
    let options = ClickOptions::builder().button(MouseButton::Right).build();
    button
        .click(Some(options))
        .await
        .expect("Failed to right-click in Firefox");

    let result = page.locator("#result").await.inner_text().await.unwrap();
    assert!(
        result.contains("contextmenu") || result.contains("right") || result.contains("auxclick"),
        "Firefox should register right click: {}",
        result
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_click_options_webkit() {
    common::init_tracing();
    // Cross-browser test: WebKit
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/click_options.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let button = page.locator("#button").await;
    let options = ClickOptions::builder().button(MouseButton::Right).build();
    button
        .click(Some(options))
        .await
        .expect("Failed to right-click in WebKit");

    let result = page.locator("#result").await.inner_text().await.unwrap();
    assert!(
        result.contains("contextmenu") || result.contains("right") || result.contains("auxclick"),
        "WebKit should register right click: {}",
        result
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
