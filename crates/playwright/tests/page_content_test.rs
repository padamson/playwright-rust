// Integration tests for page.content()
//
// Tests the page.content() method which retrieves the full HTML content of the page.
// See: https://playwright.dev/docs/api/class-page#page-content

use playwright_rs::protocol::Playwright;

mod common;

#[tokio::test]
async fn test_page_content_basic() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to a page with known HTML content
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Page</title>
</head>
<body>
    <h1 id="heading">Hello World</h1>
    <p>This is a test paragraph.</p>
</body>
</html>"#;

    // Use data URL to load the HTML
    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    page.goto(&data_url, None)
        .await
        .expect("Failed to navigate");

    // Get the page content
    let content = page.content().await.expect("Failed to get page content");

    // Verify the content contains expected elements
    assert!(
        content.contains("<!DOCTYPE html>") || content.to_lowercase().contains("<!doctype html>"),
        "Content should include DOCTYPE declaration"
    );
    assert!(
        content.contains("<html"),
        "Content should include <html> tag"
    );
    assert!(
        content.contains("<head"),
        "Content should include <head> tag"
    );
    assert!(
        content.contains("<title>Test Page</title>"),
        "Content should include <title> tag with text"
    );
    assert!(
        content.contains("<body"),
        "Content should include <body> tag"
    );
    assert!(
        content.contains("Hello World"),
        "Content should include body text"
    );

    tracing::info!("✓ page.content() returns full HTML including DOCTYPE");

    // Cleanup
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_content_empty_page() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Get content of about:blank
    let content = page.content().await.expect("Failed to get page content");

    // about:blank should still have basic HTML structure
    assert!(
        content.contains("<html") || content.contains("<HTML"),
        "Even about:blank has HTML structure"
    );

    tracing::info!("✓ page.content() works on about:blank");

    // Cleanup
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_content_with_dynamic_changes() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to a simple page
    let html = r#"<!DOCTYPE html>
<html>
<head><title>Dynamic Test</title></head>
<body>
    <div id="content">Original</div>
</body>
</html>"#;

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    page.goto(&data_url, None)
        .await
        .expect("Failed to navigate");

    // Modify the DOM using JavaScript
    page.evaluate_expression("document.getElementById('content').textContent = 'Modified'")
        .await
        .expect("Failed to evaluate script");

    // Get the updated content
    let content = page.content().await.expect("Failed to get page content");

    // Verify the content reflects the DOM changes
    assert!(
        content.contains("Modified"),
        "Content should reflect DOM changes"
    );
    assert!(
        !content.contains(">Original<"),
        "Content should not contain old text"
    );

    tracing::info!("✓ page.content() reflects dynamic DOM changes");

    // Cleanup
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_content_cross_browser() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Cross-Browser Test</title></head>
<body><h1>Test</h1></body>
</html>"#;

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));

    // Test on Chromium
    {
        let browser = playwright
            .chromium()
            .launch()
            .await
            .expect("Failed to launch Chromium");
        let page = browser.new_page().await.expect("Failed to create page");
        page.goto(&data_url, None)
            .await
            .expect("Failed to navigate");

        let content = page.content().await.expect("Failed to get content");
        assert!(
            content.contains("Cross-Browser Test"),
            "Chromium: content should contain title"
        );
        browser.close().await.expect("Failed to close browser");
        tracing::info!("✓ page.content() works on Chromium");
    }

    // Test on Firefox
    {
        let browser = playwright
            .firefox()
            .launch()
            .await
            .expect("Failed to launch Firefox");
        let page = browser.new_page().await.expect("Failed to create page");
        page.goto(&data_url, None)
            .await
            .expect("Failed to navigate");

        let content = page.content().await.expect("Failed to get content");
        assert!(
            content.contains("Cross-Browser Test"),
            "Firefox: content should contain title"
        );
        browser.close().await.expect("Failed to close browser");
        tracing::info!("✓ page.content() works on Firefox");
    }

    // Test on WebKit
    {
        let browser = playwright
            .webkit()
            .launch()
            .await
            .expect("Failed to launch WebKit");
        let page = browser.new_page().await.expect("Failed to create page");
        page.goto(&data_url, None)
            .await
            .expect("Failed to navigate");

        let content = page.content().await.expect("Failed to get content");
        assert!(
            content.contains("Cross-Browser Test"),
            "WebKit: content should contain title"
        );
        browser.close().await.expect("Failed to close browser");
        tracing::info!("✓ page.content() works on WebKit");
    }
}
