// Integration tests for checkbox and hover interactions
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - check() on unchecked checkbox
// - check() is idempotent (already checked)
// - uncheck() on checked checkbox
// - uncheck() is idempotent (already unchecked)
// - check() on radio button
// - hover() triggers CSS :hover state

mod test_server;

use playwright_rs::protocol::Playwright;
use test_server::TestServer;

#[tokio::test]
async fn test_check_unchecked_checkbox() {
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

    page.goto(&format!("{}/checkbox.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test: Check an unchecked checkbox
    let checkbox = page.locator("#checkbox").await;

    // Verify it's initially unchecked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(!is_checked);

    // Check the checkbox
    checkbox
        .check(None)
        .await
        .expect("Failed to check checkbox");

    // Verify it's now checked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(is_checked);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_check_is_idempotent() {
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

    page.goto(&format!("{}/checkbox.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let checkbox = page.locator("#checked-checkbox").await;

    // Verify it's already checked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(is_checked);

    // Check it again (should be no-op)
    checkbox
        .check(None)
        .await
        .expect("Failed to check checkbox");

    // Verify it's still checked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(is_checked);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_uncheck_checked_checkbox() {
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

    page.goto(&format!("{}/checkbox.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let checkbox = page.locator("#checked-checkbox").await;

    // Verify it's initially checked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(is_checked);

    // Uncheck the checkbox
    checkbox
        .uncheck(None)
        .await
        .expect("Failed to uncheck checkbox");

    // Verify it's now unchecked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(!is_checked);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_uncheck_is_idempotent() {
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

    page.goto(&format!("{}/checkbox.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let checkbox = page.locator("#checkbox").await;

    // Verify it's initially unchecked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(!is_checked);

    // Uncheck it again (should be no-op)
    checkbox
        .uncheck(None)
        .await
        .expect("Failed to uncheck checkbox");

    // Verify it's still unchecked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(!is_checked);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_check_radio_button() {
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

    page.goto(&format!("{}/checkbox.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test: Check a radio button
    let radio = page.locator("#radio1").await;

    // Verify it's initially unchecked
    let is_checked = radio.is_checked().await.expect("Failed to check state");
    assert!(!is_checked);

    // Check the radio button
    radio.check(None).await.expect("Failed to check radio");

    // Verify it's now checked
    let is_checked = radio.is_checked().await.expect("Failed to check state");
    assert!(is_checked);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_hover() {
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

    page.goto(&format!("{}/hover.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test: Hover shows hidden element
    let button = page.locator("#hover-button").await;
    let tooltip = page.locator("#tooltip").await;

    // Verify tooltip is initially hidden
    let is_visible = tooltip
        .is_visible()
        .await
        .expect("Failed to check visibility");
    assert!(!is_visible);

    // Hover over the button
    button.hover(None).await.expect("Failed to hover");

    // Verify tooltip is now visible
    let is_visible = tooltip
        .is_visible()
        .await
        .expect("Failed to check visibility");
    assert!(is_visible);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// Cross-browser tests

#[tokio::test]
async fn test_check_firefox() {
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

    page.goto(&format!("{}/checkbox.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let checkbox = page.locator("#checkbox").await;
    checkbox
        .check(None)
        .await
        .expect("Failed to check checkbox");

    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(is_checked);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_hover_webkit() {
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

    page.goto(&format!("{}/hover.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let button = page.locator("#hover-button").await;
    button.hover(None).await.expect("Failed to hover");

    // Verify hover worked (tooltip should be visible)
    let tooltip = page.locator("#tooltip").await;
    let is_visible = tooltip
        .is_visible()
        .await
        .expect("Failed to check visibility");
    assert!(is_visible);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
