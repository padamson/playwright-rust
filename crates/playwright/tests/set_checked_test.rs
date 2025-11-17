// Integration tests for Locator.set_checked() convenience method
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - set_checked(true) calls check()
// - set_checked(false) calls uncheck()
// - Works with checkboxes and radio buttons
// - Cross-browser compatibility

mod test_server;

use playwright_rs::protocol::Playwright;
use test_server::TestServer;

#[tokio::test]
async fn test_set_checked_true_on_checkbox() {
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

    // Test: set_checked(true) should check the checkbox
    let checkbox = page.locator("#checkbox").await;

    // Verify it starts unchecked
    let initially_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(!initially_checked, "Checkbox should start unchecked");

    // Set to checked
    checkbox
        .set_checked(true, None)
        .await
        .expect("Failed to set checked");

    // Verify it's now checked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(
        is_checked,
        "Checkbox should be checked after set_checked(true)"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_set_checked_false_on_checkbox() {
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

    // Test: set_checked(false) should uncheck the checkbox
    let checkbox = page.locator("#checkbox").await;

    // First check it
    checkbox.check(None).await.expect("Failed to check");
    let initially_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(initially_checked, "Checkbox should be checked");

    // Set to unchecked
    checkbox
        .set_checked(false, None)
        .await
        .expect("Failed to set unchecked");

    // Verify it's now unchecked
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(
        !is_checked,
        "Checkbox should be unchecked after set_checked(false)"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_set_checked_idempotent() {
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

    // Test: set_checked() should be idempotent
    let checkbox = page.locator("#checkbox").await;

    // Set to checked twice
    checkbox
        .set_checked(true, None)
        .await
        .expect("Failed to set checked");
    checkbox
        .set_checked(true, None)
        .await
        .expect("Failed to set checked again");

    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(is_checked, "Checkbox should still be checked");

    // Set to unchecked twice
    checkbox
        .set_checked(false, None)
        .await
        .expect("Failed to set unchecked");
    checkbox
        .set_checked(false, None)
        .await
        .expect("Failed to set unchecked again");

    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(!is_checked, "Checkbox should still be unchecked");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_set_checked_on_radio_button() {
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

    // Test: set_checked() works on radio buttons
    let radio1 = page.locator("#radio1").await;
    let radio2 = page.locator("#radio2").await;

    // Set radio1 to checked
    radio1
        .set_checked(true, None)
        .await
        .expect("Failed to set radio1 checked");

    let radio1_checked = radio1
        .is_checked()
        .await
        .expect("Failed to check radio1 state");
    assert!(radio1_checked, "Radio1 should be checked");

    // Set radio2 to checked (should uncheck radio1)
    radio2
        .set_checked(true, None)
        .await
        .expect("Failed to set radio2 checked");

    let radio1_checked = radio1
        .is_checked()
        .await
        .expect("Failed to check radio1 state");
    let radio2_checked = radio2
        .is_checked()
        .await
        .expect("Failed to check radio2 state");
    assert!(!radio1_checked, "Radio1 should be unchecked");
    assert!(radio2_checked, "Radio2 should be checked");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_set_checked_with_options() {
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

    // Test: set_checked() accepts CheckOptions
    let checkbox = page.locator("#checkbox").await;

    // Use timeout option (10 seconds in milliseconds)
    let options = playwright_rs::protocol::CheckOptions {
        timeout: Some(10000.0),
        ..Default::default()
    };

    checkbox
        .set_checked(true, Some(options))
        .await
        .expect("Failed to set checked with options");

    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(is_checked, "Checkbox should be checked");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// Cross-browser tests

#[tokio::test]
async fn test_set_checked_firefox() {
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

    // Test set_checked on Firefox
    let checkbox = page.locator("#checkbox").await;

    checkbox
        .set_checked(true, None)
        .await
        .expect("Failed to set checked");
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(is_checked, "Checkbox should be checked in Firefox");

    checkbox
        .set_checked(false, None)
        .await
        .expect("Failed to set unchecked");
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(!is_checked, "Checkbox should be unchecked in Firefox");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_set_checked_webkit() {
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

    page.goto(&format!("{}/checkbox.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test set_checked on WebKit
    let checkbox = page.locator("#checkbox").await;

    checkbox
        .set_checked(true, None)
        .await
        .expect("Failed to set checked");
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(is_checked, "Checkbox should be checked in WebKit");

    checkbox
        .set_checked(false, None)
        .await
        .expect("Failed to set unchecked");
    let is_checked = checkbox.is_checked().await.expect("Failed to check state");
    assert!(!is_checked, "Checkbox should be unchecked in WebKit");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
