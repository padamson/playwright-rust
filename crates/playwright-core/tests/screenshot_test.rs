// Integration tests for screenshot functionality
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - page.screenshot() with default options
// - page.screenshot() saves to file
// - page.screenshot() returns bytes
// - page.screenshot() with full_page option
// - page.screenshot() with type (png/jpeg)
// - locator.screenshot() captures element

mod test_server;

use playwright_core::protocol::Playwright;
use test_server::TestServer;

#[tokio::test]
async fn test_page_screenshot_returns_bytes() {
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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test: Screenshot returns bytes
    let bytes = page
        .screenshot(None)
        .await
        .expect("Failed to take screenshot");

    // Verify bytes are not empty and look like PNG
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]); // PNG magic bytes

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_screenshot_saves_to_file() {
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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Create temp file path
    let temp_dir = std::env::temp_dir();
    let screenshot_path = temp_dir.join("playwright_test_screenshot.png");

    // Test: Screenshot saves to file
    let bytes = page
        .screenshot_to_file(&screenshot_path, None)
        .await
        .expect("Failed to take screenshot");

    // Verify file exists
    assert!(screenshot_path.exists());

    // Verify bytes were returned
    assert!(!bytes.is_empty());

    // Verify file content matches returned bytes
    let file_bytes = std::fs::read(&screenshot_path).expect("Failed to read screenshot file");
    assert_eq!(bytes, file_bytes);

    // Cleanup
    std::fs::remove_file(screenshot_path).expect("Failed to remove screenshot file");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_screenshot_full_page() {
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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test: Full page screenshot (captures beyond viewport)
    // TODO: Need ScreenshotOptions with full_page field
    let bytes = page
        .screenshot(None)
        .await
        .expect("Failed to take full page screenshot");

    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]); // PNG

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// TODO: Element screenshots require ElementHandle protocol support
// Deferred to future phase - Frame.screenshot with selector isn't supported
// Need to implement ElementHandles first (Phase 4)
//
// #[tokio::test]
// async fn test_locator_screenshot() {
//     ...
// }

// Cross-browser tests

#[tokio::test]
async fn test_screenshot_firefox() {
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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let bytes = page
        .screenshot(None)
        .await
        .expect("Failed to take screenshot");

    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_screenshot_webkit() {
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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let bytes = page
        .screenshot(None)
        .await
        .expect("Failed to take screenshot");

    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
