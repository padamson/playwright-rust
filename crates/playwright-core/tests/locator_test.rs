// Integration tests for Locator functionality
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - Locator creation (page.locator)
// - Locator chaining (first, last, nth, locator)
// - Query methods (count, text_content, inner_text, inner_html, get_attribute)
// - State queries (is_visible, is_enabled, is_checked, is_editable)

use playwright_core::protocol::Playwright;

#[tokio::test]
async fn test_locator_creation() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test: Create a locator
    let heading = page.locator("h1").await;

    // Locator should be created (doesn't execute until action)
    assert_eq!(heading.selector(), "h1");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_count() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test: Count elements
    let paragraphs = page.locator("p").await;
    let count = paragraphs.count().await.expect("Failed to get count");

    // example.com has at least 1 paragraph
    assert!(count >= 1);

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_text_content() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test: Get text content
    let heading = page.locator("h1").await;
    let text = heading
        .text_content()
        .await
        .expect("Failed to get text content");

    // example.com has "Example Domain" heading
    assert!(text.is_some());
    assert!(text.unwrap().contains("Example Domain"));

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_chaining_first() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test: Get first paragraph
    let paragraphs = page.locator("p").await;
    let first = paragraphs.first();

    assert_eq!(first.selector(), "p >> nth=0");

    let text = first
        .text_content()
        .await
        .expect("Failed to get text content");
    assert!(text.is_some());

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_chaining_last() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test: Get last paragraph
    let paragraphs = page.locator("p").await;
    let last = paragraphs.last();

    assert_eq!(last.selector(), "p >> nth=-1");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_chaining_nth() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test: Get nth element
    let paragraphs = page.locator("p").await;
    let second = paragraphs.nth(1);

    assert_eq!(second.selector(), "p >> nth=1");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_nested() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test: Nested locators
    let body = page.locator("body").await;
    let heading = body.locator("h1");

    assert_eq!(heading.selector(), "body >> h1");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_inner_text() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test: Get visible text
    let heading = page.locator("h1").await;
    let text = heading
        .inner_text()
        .await
        .expect("Failed to get inner text");

    assert!(text.contains("Example Domain"));

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_is_visible() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test: Check visibility
    let heading = page.locator("h1").await;
    let visible = heading
        .is_visible()
        .await
        .expect("Failed to check visibility");

    assert!(visible);

    browser.close().await.expect("Failed to close browser");
}

// Cross-browser tests

#[tokio::test]
async fn test_locator_firefox() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test locator creation and text content
    let heading = page.locator("h1").await;
    let text = heading
        .text_content()
        .await
        .expect("Failed to get text content");

    assert!(text.is_some());
    assert!(text.unwrap().contains("Example Domain"));

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_webkit() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Test locator creation and visibility
    let heading = page.locator("h1").await;
    let visible = heading
        .is_visible()
        .await
        .expect("Failed to check visibility");

    assert!(visible);

    browser.close().await.expect("Failed to close browser");
}
