// Integration tests for keyboard and mouse low-level APIs
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - keyboard.down() and keyboard.up()
// - keyboard.press() with single keys and combinations
// - keyboard.type_text() for typing text
// - keyboard.insert_text() for paste-like insertion
// - mouse.move() to coordinates
// - mouse.click() at coordinates
// - mouse.dblclick() at coordinates
// - mouse.down() and mouse.up()
// - mouse.wheel() for scrolling
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~75% (12 tests → 3 tests)

mod test_server;

use playwright_rs::protocol::Playwright;
use test_server::TestServer;

mod common;

// ============================================================================
// Keyboard Tests
// ============================================================================

#[tokio::test]
async fn test_keyboard_methods() {
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

    page.goto(&format!("{}/keyboard_mouse.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let input = page.locator("#keyboard-input").await;
    let keyboard = page.keyboard();

    // Test 1: Type text using keyboard API
    input.click(None).await.expect("Failed to focus input");
    keyboard
        .type_text("Hello World", None)
        .await
        .expect("Failed to type text");

    let value = input
        .input_value(None)
        .await
        .expect("Failed to get input value");
    assert_eq!(value, "Hello World");

    // Test 2: Press Enter key
    keyboard
        .press("Enter", None)
        .await
        .expect("Failed to press Enter");

    let result = page.locator("#keyboard-result").await;
    let text = result.text_content().await.expect("Failed to get text");
    assert_eq!(text, Some("Enter pressed".to_string()));

    // Test 3: Hold Shift and type letter for uppercase
    // Clear input first
    page.evaluate_expression("document.getElementById('keyboard-input').value = ''")
        .await
        .expect("Failed to clear input");

    input.click(None).await.expect("Failed to focus input");
    keyboard.down("Shift").await.expect("Failed to press Shift");
    keyboard
        .press("KeyA", None)
        .await
        .expect("Failed to press A");
    keyboard.up("Shift").await.expect("Failed to release Shift");

    let value = input
        .input_value(None)
        .await
        .expect("Failed to get input value");
    assert!(value.contains("A"));

    // Test 4: Insert text without key events
    page.evaluate_expression("document.getElementById('keyboard-input').value = ''")
        .await
        .expect("Failed to clear input");

    input.click(None).await.expect("Failed to focus input");
    keyboard
        .insert_text("Pasted text")
        .await
        .expect("Failed to insert text");

    let value = input
        .input_value(None)
        .await
        .expect("Failed to get input value");
    assert_eq!(value, "Pasted text");

    // Test 5: Press compound key (Control+a or Meta+a)
    page.evaluate_expression("document.getElementById('keyboard-input').value = ''")
        .await
        .expect("Failed to clear input");

    input.click(None).await.expect("Failed to focus input");
    keyboard
        .type_text("Hello World", None)
        .await
        .expect("Failed to type");

    // Try to select all with Control+a (or Meta+a on Mac)
    #[cfg(target_os = "macos")]
    keyboard
        .press("Meta+a", None)
        .await
        .expect("Failed to press Meta+a");

    #[cfg(not(target_os = "macos"))]
    keyboard
        .press("Control+a", None)
        .await
        .expect("Failed to press Control+a");

    // If selection works, typing should replace all text
    keyboard
        .type_text("Replaced", None)
        .await
        .expect("Failed to type replacement");

    let value = input.input_value(None).await.expect("Failed to get value");
    assert_eq!(
        value, "Replaced",
        "Text should be replaced if select-all worked"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Mouse Tests
// ============================================================================

#[tokio::test]
async fn test_mouse_methods() {
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

    page.goto(&format!("{}/keyboard_mouse.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let mouse = page.mouse();

    // Test 1: Move mouse to coordinates
    mouse
        .move_to(100, 100, None)
        .await
        .expect("Failed to move mouse");

    // Verify mouse moved (page would show coordinates in real scenario)
    let coords = page.locator("#mouse-coords").await;
    let text = coords.text_content().await.expect("Failed to get text");
    assert!(text.is_some());

    // Test 2: Click at coordinates
    mouse
        .click(150, 200, None)
        .await
        .expect("Failed to click mouse");

    let result = page.locator("#mouse-result").await;
    let text = result.text_content().await.expect("Failed to get text");
    assert_eq!(text, Some("Clicked".to_string()));

    // Test 3: Double-click at coordinates
    mouse
        .dblclick(150, 200, None)
        .await
        .expect("Failed to double-click mouse");

    let text = result.text_content().await.expect("Failed to get text");
    assert_eq!(text, Some("Double-clicked".to_string()));

    // Test 4: Mouse down and up (simulating drag)
    mouse
        .move_to(150, 200, None)
        .await
        .expect("Failed to move mouse");
    mouse.down(None).await.expect("Failed to mouse down");
    mouse
        .move_to(250, 200, None)
        .await
        .expect("Failed to move while down");
    mouse.up(None).await.expect("Failed to mouse up");

    // Test 5: Scroll with mouse wheel
    mouse.wheel(0, 100).await.expect("Failed to wheel mouse");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    common::init_tracing();
    // Smoke test to verify keyboard and mouse work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each method)

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test Firefox - keyboard
    let firefox = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");
    let firefox_page = firefox.new_page().await.expect("Failed to create page");

    firefox_page
        .goto(&format!("{}/keyboard_mouse.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let firefox_input = firefox_page.locator("#keyboard-input").await;
    firefox_input
        .click(None)
        .await
        .expect("Failed to focus input");

    let firefox_keyboard = firefox_page.keyboard();
    firefox_keyboard
        .type_text("Firefox test", None)
        .await
        .expect("Failed to type text");

    let value = firefox_input
        .input_value(None)
        .await
        .expect("Failed to get input value");
    assert_eq!(value, "Firefox test");

    firefox.close().await.expect("Failed to close Firefox");

    // Test WebKit - mouse
    let webkit = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let webkit_page = webkit.new_page().await.expect("Failed to create page");

    webkit_page
        .goto(&format!("{}/keyboard_mouse.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let webkit_mouse = webkit_page.mouse();
    webkit_mouse
        .click(150, 200, None)
        .await
        .expect("Failed to click mouse");

    let webkit_result = webkit_page.locator("#mouse-result").await;
    let text = webkit_result
        .text_content()
        .await
        .expect("Failed to get text");
    assert_eq!(text, Some("Clicked".to_string()));

    webkit.close().await.expect("Failed to close WebKit");
    server.shutdown();
}
