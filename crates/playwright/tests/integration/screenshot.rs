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

use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;
use playwright_rs::protocol::screenshot::{ScreenshotClip, ScreenshotOptions, ScreenshotType};

#[tokio::test]
async fn test_page_screenshot_returns_bytes() {
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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test: Full page screenshot (captures beyond viewport)
    use playwright_rs::protocol::ScreenshotOptions;
    let options = ScreenshotOptions::builder().full_page(true).build();
    let bytes = page
        .screenshot(Some(options))
        .await
        .expect("Failed to take full page screenshot");

    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]); // PNG

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_screenshot() {
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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test: Element screenshot via locator
    let heading = page.locator("h1").await;
    let bytes = heading
        .screenshot(None)
        .await
        .expect("Failed to take locator screenshot");

    // Verify bytes are not empty and look like PNG
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]); // PNG magic bytes

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// Cross-browser tests

#[tokio::test]
async fn test_screenshot_firefox() {
    crate::common::init_tracing();
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
    crate::common::init_tracing();
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

// ============================================================================
// Merged from: screenshot_options_test.rs
// ============================================================================

// Integration tests for Screenshot Options (Phase 4, Slice 2)
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - JPEG format with quality option
// - Full-page screenshots
// - Clip region screenshots
// - PNG vs JPEG format differences
// - Cross-browser compatibility
//
// Note: Tests are combined where possible to reduce browser launches and improve speed

#[tokio::test]
async fn test_screenshot_all_page_options() {
    crate::common::init_tracing();
    // Combined test: All page screenshot options in one browser session
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

    // Test 1: JPEG with quality
    let options = ScreenshotOptions::builder()
        .screenshot_type(ScreenshotType::Jpeg)
        .quality(80)
        .build();
    let bytes = page
        .screenshot(Some(options))
        .await
        .expect("Failed to take JPEG screenshot");
    assert!(!bytes.is_empty(), "JPEG screenshot should not be empty");
    assert_eq!(
        &bytes[0..2],
        &[0xFF, 0xD8],
        "Screenshot should be JPEG format"
    );

    // Test 2: Explicit PNG format
    let options = ScreenshotOptions::builder()
        .screenshot_type(ScreenshotType::Png)
        .build();
    let bytes = page
        .screenshot(Some(options))
        .await
        .expect("Failed to take PNG screenshot");
    assert!(!bytes.is_empty(), "PNG screenshot should not be empty");
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47], "Should be PNG");

    // Test 3: Full page screenshot
    let options = ScreenshotOptions::builder().full_page(true).build();
    let bytes = page
        .screenshot(Some(options))
        .await
        .expect("Failed to take full page screenshot");
    assert!(
        !bytes.is_empty(),
        "Full page screenshot should not be empty"
    );
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47], "Should be PNG");

    // Test 4: Clip region
    let clip = ScreenshotClip {
        x: 10.0,
        y: 10.0,
        width: 200.0,
        height: 100.0,
    };
    let options = ScreenshotOptions::builder().clip(clip).build();
    let bytes = page
        .screenshot(Some(options))
        .await
        .expect("Failed to take clip screenshot");
    assert!(!bytes.is_empty(), "Clip screenshot should not be empty");
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47], "Should be PNG");

    // Test 5: Omit background (transparent PNG)
    let options = ScreenshotOptions::builder().omit_background(true).build();
    let bytes = page
        .screenshot(Some(options))
        .await
        .expect("Failed to take transparent screenshot");
    assert!(
        !bytes.is_empty(),
        "Transparent screenshot should not be empty"
    );
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47], "Should be PNG");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_screenshot_element_and_locator_with_options() {
    crate::common::init_tracing();
    // Combined test: Element and locator screenshots with options
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

    // Test 1: ElementHandle screenshot with JPEG
    let element = page
        .query_selector("h1")
        .await
        .expect("Failed to query selector")
        .expect("h1 should exist");

    let options = ScreenshotOptions::builder()
        .screenshot_type(ScreenshotType::Jpeg)
        .quality(90)
        .build();
    let bytes = element
        .screenshot(Some(options))
        .await
        .expect("Failed to take element screenshot");
    assert!(!bytes.is_empty(), "Element screenshot should not be empty");
    assert_eq!(&bytes[0..2], &[0xFF, 0xD8], "Should be JPEG");

    // Test 2: Locator screenshot with options
    let locator = page.locator("h1").await;
    let options = ScreenshotOptions::builder()
        .screenshot_type(ScreenshotType::Jpeg)
        .quality(85)
        .build();
    let bytes = locator
        .screenshot(Some(options))
        .await
        .expect("Failed to take locator screenshot");
    assert!(!bytes.is_empty(), "Locator screenshot should not be empty");
    assert_eq!(&bytes[0..2], &[0xFF, 0xD8], "Should be JPEG");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_screenshot_options_firefox() {
    crate::common::init_tracing();
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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let options = ScreenshotOptions::builder()
        .screenshot_type(ScreenshotType::Jpeg)
        .quality(80)
        .build();
    let bytes = page
        .screenshot(Some(options))
        .await
        .expect("Failed to take JPEG screenshot");
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..2], &[0xFF, 0xD8]);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_screenshot_options_webkit() {
    crate::common::init_tracing();
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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let options = ScreenshotOptions::builder()
        .screenshot_type(ScreenshotType::Jpeg)
        .quality(80)
        .build();
    let bytes = page
        .screenshot(Some(options))
        .await
        .expect("Failed to take JPEG screenshot");
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..2], &[0xFF, 0xD8]);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
