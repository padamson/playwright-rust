// Integration tests for Assertions (Phase 5, Slice 1)
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - expect().to_be_visible() - auto-retry until visible
// - expect().to_be_hidden() - auto-retry until hidden
// - expect().not().to_be_visible() - negation support
// - Timeout behavior
// - Cross-browser compatibility
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~73% (11 tests → 3 tests)

mod test_server;

use playwright_rs::{expect, protocol::Playwright};
use test_server::TestServer;

mod common;

// ============================================================================
// to_be_visible() Assertions
// ============================================================================

#[tokio::test]
async fn test_to_be_visible_assertions() {
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

    page.goto(&format!("{}/button.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Element that is already visible should pass immediately
    let button = page.locator("#btn").await;
    expect(button)
        .to_be_visible()
        .await
        .expect("Button should be visible");

    // Test 2: Negation - element should NOT be visible
    let nonexistent = page.locator("#does-not-exist").await;
    expect(nonexistent.clone())
        .not()
        .to_be_visible()
        .await
        .expect("Nonexistent element should NOT be visible");

    // Test 3: Should timeout if element never appears
    let result = expect(nonexistent)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_be_visible()
        .await;

    assert!(result.is_err(), "Should timeout for nonexistent element");
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("timeout") || error_message.contains("Assertion"),
        "Error message should mention timeout: {}",
        error_message
    );

    // Test 4: Auto-retry - assertion should wait and retry until element becomes visible
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    page.evaluate_expression(
        r#"
        const div = document.createElement('div');
        div.id = 'delayed-element';
        div.textContent = 'I will appear!';
        div.style.display = 'none';
        document.body.appendChild(div);

        setTimeout(() => {
            div.style.display = 'block';
        }, 100);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let delayed = page.locator("#delayed-element").await;
    let start = std::time::Instant::now();

    expect(delayed)
        .to_be_visible()
        .await
        .expect("Delayed element should eventually be visible");

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 100,
        "Should have waited at least 100ms, but was {:?}",
        elapsed
    );

    // Test 5: Custom timeout - element that appears after 200ms
    page.evaluate_expression(
        r#"
        const div = document.createElement('div');
        div.id = 'slow-element';
        div.textContent = 'Slow element';
        div.style.display = 'none';
        document.body.appendChild(div);

        setTimeout(() => {
            div.style.display = 'block';
        }, 200);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let slow = page.locator("#slow-element").await;
    expect(slow)
        .to_be_visible()
        .await
        .expect("Should wait up to 5s by default");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// to_be_hidden() Assertions
// ============================================================================

#[tokio::test]
async fn test_to_be_hidden_assertions() {
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

    page.goto(&format!("{}/button.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Element that doesn't exist should be considered hidden
    let nonexistent = page.locator("#does-not-exist").await;
    expect(nonexistent)
        .to_be_hidden()
        .await
        .expect("Nonexistent element should be hidden");

    // Test 2: Auto-retry - assertion should wait until element becomes hidden
    page.evaluate_expression(
        r#"
        const btn = document.getElementById('btn');
        setTimeout(() => {
            btn.style.display = 'none';
        }, 100);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let button = page.locator("#btn").await;
    let start = std::time::Instant::now();

    expect(button)
        .to_be_hidden()
        .await
        .expect("Button should eventually be hidden");

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 100,
        "Should have waited at least 100ms, but was {:?}",
        elapsed
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    common::init_tracing();
    // Smoke test to verify assertions work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each assertion)

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
        .goto(&format!("{}/button.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let firefox_button = firefox_page.locator("#btn").await;
    expect(firefox_button)
        .to_be_visible()
        .await
        .expect("Button should be visible in Firefox");

    firefox.close().await.expect("Failed to close Firefox");

    // Test WebKit
    let webkit = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let webkit_page = webkit.new_page().await.expect("Failed to create page");

    webkit_page
        .goto(&format!("{}/button.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let webkit_nonexistent = webkit_page.locator("#does-not-exist").await;
    expect(webkit_nonexistent)
        .to_be_hidden()
        .await
        .expect("Nonexistent element should be hidden in WebKit");

    // Test auto-retry in WebKit
    webkit_page
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    webkit_page
        .evaluate_expression(
            r#"
        const div = document.createElement('div');
        div.id = 'delayed-webkit';
        div.textContent = 'WebKit element';
        div.style.display = 'none';
        document.body.appendChild(div);

        setTimeout(() => {
            div.style.display = 'block';
        }, 100);
        "#,
        )
        .await
        .expect("Failed to inject script");

    let webkit_delayed = webkit_page.locator("#delayed-webkit").await;
    expect(webkit_delayed)
        .to_be_visible()
        .await
        .expect("Auto-retry should work in WebKit");

    webkit.close().await.expect("Failed to close WebKit");
    server.shutdown();
}
