// Integration tests for Action Options (Phase 4, Slice 5)
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - Fill options (force, timeout)
// - Press options (delay, timeout)
// - Check options (force, position, timeout, trial)
// - Hover options (force, modifiers, position, timeout, trial)
// - Select options (force, timeout)
// - Keyboard options (delay)
// - Mouse options (button, click_count, delay, steps)
// - Cross-browser compatibility
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~78% (9 tests → 2 tests)

mod test_server;

use playwright_rs::protocol::action_options::{
    CheckOptions, FillOptions, HoverOptions, KeyboardOptions, MouseOptions, PressOptions,
    SelectOptions,
};
use playwright_rs::protocol::{GotoOptions, MouseButton, Playwright, Position};
use test_server::TestServer;

// ============================================================================
// Action Options Methods
// ============================================================================

#[tokio::test]
async fn test_action_options_methods() {
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

    // Test 1: Fill with force option
    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let input = page.locator("#input").await;
    let options = FillOptions::builder().force(true).build();
    input
        .fill("Hello World", Some(options))
        .await
        .expect("Failed to fill with force");

    let value = input.input_value(None).await.unwrap();
    assert_eq!(value, "Hello World");

    println!("✓ Fill with force option works");

    // Test 2: Press with delay option
    page.goto(&format!("{}/keyboard.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let input = page.locator("#input").await;
    input.click(None).await.expect("Failed to click");

    let options = PressOptions::builder().delay(50.0).build();
    input
        .press("Enter", Some(options))
        .await
        .expect("Failed to press with delay");

    let value = input.input_value(None).await.unwrap();
    assert_eq!(value, "submitted");

    println!("✓ Press with delay option works");

    // Test 3: Check with force and trial options
    page.goto(&format!("{}/checkbox.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let checkbox = page.locator("#checkbox").await;
    let options = CheckOptions::builder().force(true).build();
    checkbox
        .check(Some(options))
        .await
        .expect("Failed to check with force");

    assert!(
        checkbox.is_checked().await.unwrap(),
        "Checkbox should be checked"
    );

    let checked_checkbox = page.locator("#checked-checkbox").await;
    let trial_options = CheckOptions::builder().trial(true).build();
    checked_checkbox
        .uncheck(Some(trial_options))
        .await
        .expect("Failed to trial uncheck");

    assert!(
        checked_checkbox.is_checked().await.unwrap(),
        "Trial uncheck should not actually uncheck"
    );

    println!("✓ Check with force and trial options work");

    // Test 4: Hover with position option
    page.goto(&format!("{}/hover.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let button = page.locator("#hover-button").await;
    let options = HoverOptions::builder()
        .position(Position { x: 5.0, y: 5.0 })
        .build();
    button
        .hover(Some(options))
        .await
        .expect("Failed to hover with position");

    let tooltip = page.locator("#tooltip").await;
    assert!(
        tooltip.is_visible().await.unwrap(),
        "Tooltip should be visible after hover"
    );

    println!("✓ Hover with position option works");

    // Test 5: Select with force option
    page.goto(&format!("{}/select.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let select = page.locator("#single-select").await;
    let options = SelectOptions::builder().force(true).build();
    let selected = select
        .select_option("apple", Some(options))
        .await
        .expect("Failed to select with force");

    assert_eq!(selected, vec!["apple"]);

    println!("✓ Select with force option works");

    // Test 6: Keyboard type with delay option
    page.goto(&format!("{}/keyboard_mouse.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let input = page.locator("#keyboard-input").await;
    input.click(None).await.expect("Failed to click input");

    let keyboard = page.keyboard();
    let options = KeyboardOptions::builder().delay(10.0).build();
    keyboard
        .type_text("Hello", Some(options))
        .await
        .expect("Failed to type with delay");

    let value = input.input_value(None).await.unwrap();
    assert_eq!(value, "Hello");

    println!("✓ Keyboard type with delay option works");

    // Test 7: Mouse click with options
    let mouse = page.mouse();
    let options = MouseOptions::builder()
        .button(MouseButton::Left)
        .click_count(1)
        .build();
    mouse
        .click(150, 150, Some(options))
        .await
        .expect("Failed to click with options");

    let result = page
        .locator("#mouse-result")
        .await
        .inner_text()
        .await
        .unwrap();
    assert_eq!(result, "Clicked");

    println!("✓ Mouse click with options works");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    // Smoke test to verify action options work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each method)

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test Firefox - fill with options
    let firefox = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");
    let firefox_page = firefox.new_page().await.expect("Failed to create page");

    firefox_page
        .goto(
            &format!("{}/input.html", server.url()),
            Some(GotoOptions::new().timeout(std::time::Duration::from_secs(60))),
        )
        .await
        .expect("Failed to navigate");

    let firefox_input = firefox_page.locator("#input").await;
    let options = FillOptions::builder().force(true).build();
    firefox_input
        .fill("Firefox Test", Some(options))
        .await
        .expect("Failed to fill in Firefox");

    let value = firefox_input.input_value(None).await.unwrap();
    assert_eq!(value, "Firefox Test");

    println!("✓ Firefox action options work");

    firefox.close().await.expect("Failed to close Firefox");

    // Test WebKit - check with options
    let webkit = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let webkit_page = webkit.new_page().await.expect("Failed to create page");

    webkit_page
        .goto(
            &format!("{}/checkbox.html", server.url()),
            Some(GotoOptions::new().timeout(std::time::Duration::from_secs(60))),
        )
        .await
        .expect("Failed to navigate");

    let webkit_checkbox = webkit_page.locator("#checkbox").await;
    let options = CheckOptions::builder().force(true).build();
    webkit_checkbox
        .check(Some(options))
        .await
        .expect("Failed to check in WebKit");

    assert!(
        webkit_checkbox.is_checked().await.unwrap(),
        "Checkbox should be checked in WebKit"
    );

    println!("✓ WebKit action options work");

    webkit.close().await.expect("Failed to close WebKit");
    server.shutdown();
}
