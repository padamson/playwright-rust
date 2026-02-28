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

use crate::test_server::TestServer;
use playwright_rs::{expect, protocol::Playwright};

// ============================================================================
// to_be_visible() Assertions
// ============================================================================

#[tokio::test]
async fn test_to_be_visible_assertions() {
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
    crate::common::init_tracing();
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

// ============================================================================
// Merged from: state_assertions_test.rs
// ============================================================================

// Integration tests for State Assertions (Phase 5, Slice 3)
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - expect().to_be_enabled() / to_be_disabled()
// - expect().to_be_checked() / to_be_unchecked()
// - expect().to_be_editable()
// - expect().to_be_focused()
// - Auto-retry behavior
// - Cross-browser compatibility
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~70% (85s → 25s for comprehensive coverage)

// ============================================================================
// Button State Assertions (enabled/disabled)
// ============================================================================

#[tokio::test]
async fn test_button_state_assertions() {
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

    // Test 1: to_be_enabled() with existing button
    page.goto(&format!("{}/button.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let button = page.locator("#btn").await;
    expect(button)
        .to_be_enabled()
        .await
        .expect("Button should be enabled");

    // Test 2: to_be_disabled() with disabled button
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    page.evaluate_expression(
        r#"
        const btn = document.createElement('button');
        btn.id = 'disabled-btn';
        btn.textContent = 'Disabled';
        btn.disabled = true;
        document.body.appendChild(btn);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let disabled_button = page.locator("#disabled-btn").await;
    expect(disabled_button.clone())
        .to_be_disabled()
        .await
        .expect("Button should be disabled");

    // Test 3: to_be_enabled() with auto-retry (delayed enable)
    page.evaluate_expression(
        r#"
        const btn = document.createElement('button');
        btn.id = 'delayed-btn';
        btn.textContent = 'Will be enabled';
        btn.disabled = true;
        document.body.appendChild(btn);

        setTimeout(() => {
            btn.disabled = false;
        }, 100);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let delayed_button = page.locator("#delayed-btn").await;
    expect(delayed_button)
        .to_be_enabled()
        .await
        .expect("Button should eventually be enabled");

    // Test 4: .not().to_be_enabled() - negation test
    expect(disabled_button.clone())
        .not()
        .to_be_enabled()
        .await
        .expect("Disabled button should NOT be enabled");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Checkbox State Assertions (checked/unchecked)
// ============================================================================

#[tokio::test]
async fn test_checkbox_state_assertions() {
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

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: to_be_checked() with checked checkbox
    page.evaluate_expression(
        r#"
        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.id = 'checked-box';
        checkbox.checked = true;
        document.body.appendChild(checkbox);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let checked_checkbox = page.locator("#checked-box").await;
    expect(checked_checkbox)
        .to_be_checked()
        .await
        .expect("Checkbox should be checked");

    // Test 2: to_be_unchecked() with unchecked checkbox
    page.evaluate_expression(
        r#"
        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.id = 'unchecked-box';
        checkbox.checked = false;
        document.body.appendChild(checkbox);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let unchecked_checkbox = page.locator("#unchecked-box").await;
    expect(unchecked_checkbox)
        .to_be_unchecked()
        .await
        .expect("Checkbox should be unchecked");

    // Test 3: to_be_checked() with auto-retry (delayed check)
    page.evaluate_expression(
        r#"
        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.id = 'delayed-checkbox';
        checkbox.checked = false;
        document.body.appendChild(checkbox);

        setTimeout(() => {
            checkbox.checked = true;
        }, 100);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let delayed_checkbox = page.locator("#delayed-checkbox").await;
    expect(delayed_checkbox)
        .to_be_checked()
        .await
        .expect("Checkbox should eventually be checked");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Editable State Assertions
// ============================================================================

#[tokio::test]
async fn test_editable_assertions() {
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

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: to_be_editable() with normal input
    page.evaluate_expression(
        r#"
        const input = document.createElement('input');
        input.type = 'text';
        input.id = 'editable-input';
        document.body.appendChild(input);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let editable_input = page.locator("#editable-input").await;
    expect(editable_input)
        .to_be_editable()
        .await
        .expect("Input should be editable");

    // Test 2: .not().to_be_editable() with readonly input
    page.evaluate_expression(
        r#"
        const input = document.createElement('input');
        input.type = 'text';
        input.id = 'readonly-input';
        input.readOnly = true;
        document.body.appendChild(input);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let readonly_input = page.locator("#readonly-input").await;
    expect(readonly_input)
        .not()
        .to_be_editable()
        .await
        .expect("Readonly input should NOT be editable");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Focus State Assertions
// ============================================================================

#[tokio::test]
async fn test_focus_assertions() {
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

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: to_be_focused() with focused input
    page.evaluate_expression(
        r#"
        const input = document.createElement('input');
        input.type = 'text';
        input.id = 'focused-input';
        document.body.appendChild(input);
        input.focus();
        "#,
    )
    .await
    .expect("Failed to inject script");

    let focused_input = page.locator("#focused-input").await;
    expect(focused_input)
        .to_be_focused()
        .await
        .expect("Input should be focused");

    // Test 2: .not().to_be_focused() with unfocused input
    page.evaluate_expression(
        r#"
        const input = document.createElement('input');
        input.type = 'text';
        input.id = 'unfocused-input';
        document.body.appendChild(input);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let unfocused_input = page.locator("#unfocused-input").await;
    expect(unfocused_input)
        .not()
        .to_be_focused()
        .await
        .expect("Input should NOT be focused");

    // Test 3: to_be_focused() with auto-retry (delayed focus)
    page.evaluate_expression(
        r#"
        const input = document.createElement('input');
        input.type = 'text';
        input.id = 'delayed-focused-input';
        document.body.appendChild(input);

        setTimeout(() => {
            input.focus();
        }, 100);
        "#,
    )
    .await
    .expect("Failed to inject script");

    let delayed_focused_input = page.locator("#delayed-focused-input").await;
    expect(delayed_focused_input)
        .to_be_focused()
        .await
        .expect("Input should eventually be focused");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_state_assertions_cross_browser_smoke() {
    // Smoke test to verify assertions work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each assertion)

    crate::common::init_tracing();
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
        .to_be_enabled()
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
        .goto(&format!("{}/button.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let webkit_button = webkit_page.locator("#btn").await;
    expect(webkit_button)
        .to_be_enabled()
        .await
        .expect("Should work in WebKit");

    webkit.close().await.expect("Failed to close WebKit");
    server.shutdown();
}

// ============================================================================
// Merged from: text_assertions_test.rs
// ============================================================================

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

// ============================================================================
// to_have_text() Assertions
// ============================================================================

#[tokio::test]
async fn test_to_have_text_assertions() {
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
async fn test_text_assertions_cross_browser_smoke() {
    // Smoke test to verify assertions work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each assertion)

    crate::common::init_tracing();
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
