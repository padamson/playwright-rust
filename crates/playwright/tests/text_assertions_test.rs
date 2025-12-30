// Integration tests for Text Assertions (Phase 5, Slice 2)
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - expect().to_have_text() - exact text match
// - expect().to_contain_text() - substring match
// - expect().to_have_value() - input value match
// - Regex pattern support for all
// - Auto-retry behavior
// - Cross-browser compatibility
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~73% (15 tests → 4 tests)

mod common;
mod test_server;

use playwright_rs::{expect, protocol::Playwright};
use test_server::TestServer;

// ============================================================================
// to_have_text() Assertions
// ============================================================================

#[tokio::test]
async fn test_to_have_text_assertions() {
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

    page.goto(&format!("{}/text.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Exact text match
    let heading = page.locator("h1").await;
    expect(heading.clone())
        .to_have_text("Welcome to Playwright")
        .await
        .expect("Heading should have exact text");

    // Test 2: Text with whitespace trimming
    let paragraph = page.locator("#whitespace").await;
    expect(paragraph)
        .to_have_text("Text with whitespace")
        .await
        .expect("Should match trimmed text");

    // Test 3: Wrong text should timeout (failure case)
    let result = expect(heading.clone())
        .with_timeout(std::time::Duration::from_millis(500))
        .to_have_text("Wrong Text")
        .await;
    assert!(result.is_err(), "Should fail for wrong text");

    // Test 4: Regex pattern should match
    expect(heading.clone())
        .to_have_text_regex(r"Welcome to .*")
        .await
        .expect("Should match regex pattern");

    // Test 5: Auto-retry behavior (delayed text change)
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    page.evaluate_expression(
        r#"
        const div = document.createElement('div');
        div.id = 'changing-text';
        div.textContent = 'Initial text';
        document.body.appendChild(div);

        setTimeout(() => {
            div.textContent = 'Changed text';
        }, 100);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let div = page.locator("#changing-text").await;
    let start = std::time::Instant::now();

    expect(div)
        .to_have_text("Changed text")
        .await
        .expect("Should eventually have changed text");

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 100,
        "Should have waited at least 100ms"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// to_contain_text() Assertions
// ============================================================================

#[tokio::test]
async fn test_to_contain_text_assertions() {
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

    page.goto(&format!("{}/text.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Substring match
    let paragraph = page.locator("#long-text").await;
    expect(paragraph.clone())
        .to_contain_text("middle of the text")
        .await
        .expect("Should contain substring");

    // Test 2: Non-existent substring should fail
    let result = expect(paragraph.clone())
        .with_timeout(std::time::Duration::from_millis(500))
        .to_contain_text("nonexistent text")
        .await;
    assert!(result.is_err(), "Should fail for missing substring");

    // Test 3: Regex pattern for substring
    expect(paragraph)
        .to_contain_text_regex(r"middle of .* text")
        .await
        .expect("Should contain regex pattern");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// to_have_value() Assertions
// ============================================================================

#[tokio::test]
async fn test_to_have_value_assertions() {
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

    page.goto(&format!("{}/text.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Input with value should match
    let input = page.locator("#name-input").await;
    expect(input.clone())
        .to_have_value("John Doe")
        .await
        .expect("Input should have value");

    // Test 2: Empty input should have empty value
    let empty_input = page.locator("#empty-input").await;
    expect(empty_input)
        .to_have_value("")
        .await
        .expect("Empty input should have empty value");

    // Test 3: Regex pattern for input value
    expect(input)
        .to_have_value_regex(r"John .*")
        .await
        .expect("Should match value regex pattern");

    // Test 4: Auto-retry behavior (delayed value change)
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    page.evaluate_expression(
        r#"
        const input = document.createElement('input');
        input.id = 'changing-input';
        input.value = 'initial';
        document.body.appendChild(input);

        setTimeout(() => {
            input.value = 'updated';
        }, 100);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let changing_input = page.locator("#changing-input").await;
    expect(changing_input)
        .to_have_value("updated")
        .await
        .expect("Should eventually have updated value");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    // Smoke test to verify assertions work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each assertion)

    common::init_tracing();
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
        .goto(&format!("{}/text.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let firefox_heading = firefox_page.locator("h1").await;
    expect(firefox_heading)
        .to_have_text("Welcome to Playwright")
        .await
        .expect("Should work in Firefox");

    firefox.close().await.expect("Failed to close Firefox");

    // Test WebKit
    let webkit = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let webkit_page = webkit.new_page().await.expect("Failed to create page");

    webkit_page
        .goto(&format!("{}/text.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let webkit_paragraph = webkit_page.locator("#long-text").await;
    expect(webkit_paragraph.clone())
        .to_contain_text("middle of the text")
        .await
        .expect("to_contain_text should work in WebKit");

    let webkit_input = webkit_page.locator("#name-input").await;
    expect(webkit_input)
        .to_have_value("John Doe")
        .await
        .expect("to_have_value should work in WebKit");

    webkit.close().await.expect("Failed to close WebKit");
    server.shutdown();
}
