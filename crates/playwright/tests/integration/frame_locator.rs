// Integration tests for FrameLocator
//
// Tests frame_locator() on Page, Locator, and FrameLocator,
// including get_by_* methods, composition (first/last/nth), owner, and nesting.

use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;

// ============================================================================
// Core: Page::frame_locator() + locator()
// ============================================================================

/// Basic FrameLocator: click a button inside an iframe
#[tokio::test]
async fn test_frame_locator_click_button() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    // Click button inside the "content" iframe
    let frame = page.frame_locator("iframe[name='content']").await;
    frame.locator("#frame-btn").click(None).await?;

    // Verify button text changed
    let text = frame.locator("#frame-btn").text_content().await?;
    assert_eq!(text, Some("clicked".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// FrameLocator reads text from inside iframe
#[tokio::test]
async fn test_frame_locator_text_content() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']").await;
    let heading = frame.locator("h1").text_content().await?;
    assert_eq!(heading, Some("Inside Frame".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// get_by_* methods
// ============================================================================

/// get_by_text inside iframe
#[tokio::test]
async fn test_frame_locator_get_by_text() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']").await;
    let btn = frame.get_by_text("Click Me", false);
    let text = btn.text_content().await?;
    assert_eq!(text, Some("Click Me".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// get_by_label inside iframe
#[tokio::test]
async fn test_frame_locator_get_by_label() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']").await;
    let input = frame.get_by_label("Email", false);
    input.fill("test@example.com", None).await?;
    let value = input.input_value(None).await?;
    assert_eq!(value, "test@example.com");

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// get_by_test_id inside iframe
#[tokio::test]
async fn test_frame_locator_get_by_test_id() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']").await;
    let btn = frame.get_by_test_id("frame-submit");
    let text = btn.text_content().await?;
    assert_eq!(text, Some("Submit".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// Nested FrameLocator
// ============================================================================

/// Nested frame_locator: iframe within iframe
#[tokio::test]
async fn test_frame_locator_nested() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto(&format!("{}/nested-iframe.html", server.url()), None)
        .await?;

    // Navigate: outer page → #outer iframe → #innermost iframe → h1
    let inner = page
        .frame_locator("#outer")
        .await
        .frame_locator("#innermost");
    let heading = inner.locator("h1").text_content().await?;
    assert_eq!(heading, Some("Inside Frame".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// owner property
// ============================================================================

/// owner() returns a Locator for the iframe element itself
#[tokio::test]
async fn test_frame_locator_owner() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']").await;
    let iframe_element = frame.owner();
    let name = iframe_element.get_attribute("name").await?;
    assert_eq!(name, Some("content".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// Locator::frame_locator()
// ============================================================================

/// frame_locator() from a Locator (scoped)
#[tokio::test]
async fn test_locator_frame_locator() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    // Use locator("body") to scope, then frame_locator into iframe
    let heading = page
        .locator("body")
        .await
        .frame_locator("iframe[name='content']")
        .locator("h1")
        .text_content()
        .await?;
    assert_eq!(heading, Some("Inside Frame".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}
