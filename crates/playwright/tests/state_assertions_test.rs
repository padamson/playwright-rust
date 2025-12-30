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

mod common;
mod test_server;

use playwright_rs::{expect, protocol::Playwright};
use test_server::TestServer;

// ============================================================================
// Button State Assertions (enabled/disabled)
// ============================================================================

#[tokio::test]
async fn test_button_state_assertions() {
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
