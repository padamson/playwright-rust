use crate::test_server::TestServer;
use playwright_rs::{expect, protocol::Playwright};

// ============================================================================
// to_be_visible() Assertions
// ============================================================================

#[tokio::test]
async fn test_to_be_visible_assertions() {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
#[ignore]
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
// Button State Assertions (enabled/disabled)
// ============================================================================

#[tokio::test]
async fn test_button_state_assertions() {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
#[ignore]
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
// to_have_text() Assertions
// ============================================================================

#[tokio::test]
async fn test_to_have_text_assertions() {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
// to_have_attribute / to_have_class / to_have_css / to_have_count
// ============================================================================

#[tokio::test]
async fn test_to_have_attribute_assertions() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<a id='link' href='/path' data-x='1'>Link</a>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let link = page.locator("#link").await;

    expect(link.clone())
        .to_have_attribute("href", "/path")
        .await
        .expect("href should match");

    expect(link.clone())
        .to_have_attribute("data-x", "1")
        .await
        .expect("data-x should match");

    expect(link.clone())
        .not()
        .to_have_attribute("href", "/other")
        .await
        .expect("href should NOT equal /other");

    expect(link.clone())
        .to_have_attribute_regex("href", r"^/p")
        .await
        .expect("href regex should match");

    let result = expect(link.clone())
        .with_timeout(std::time::Duration::from_millis(300))
        .to_have_attribute("href", "/wrong")
        .await;
    assert!(result.is_err(), "Mismatched value should time out");

    let result = expect(link)
        .with_timeout(std::time::Duration::from_millis(300))
        .to_have_attribute("nonexistent", "")
        .await;
    assert!(
        result.is_err(),
        "Missing attribute should not match empty string"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_to_have_class_assertions() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<div id='a' class='primary'>A</div><div id='b' class='primary large'>B</div>",
        None,
    )
    .await
    .expect("Failed to navigate");

    expect(page.locator("#a").await)
        .to_have_class("primary")
        .await
        .expect("Single class should match");

    expect(page.locator("#b").await)
        .to_have_class("primary large")
        .await
        .expect("Multi-class string should match exactly");

    expect(page.locator("#b").await)
        .not()
        .to_have_class("primary")
        .await
        .expect("Multi-class element should NOT equal single-class string");

    expect(page.locator("#b").await)
        .to_have_class_regex(r"\blarge\b")
        .await
        .expect("Class regex should match");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_to_have_css_assertions() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<div id='styled' style='color: rgb(255, 0, 0); font-weight: 700;'>X</div>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let el = page.locator("#styled").await;

    expect(el.clone())
        .to_have_css("color", "rgb(255, 0, 0)")
        .await
        .expect("Color should match computed style");

    expect(el.clone())
        .to_have_css("font-weight", "700")
        .await
        .expect("Font weight should be normalized to numeric");

    expect(el.clone())
        .to_have_css_regex("color", r"^rgb")
        .await
        .expect("CSS regex should match");

    let result = expect(el)
        .with_timeout(std::time::Duration::from_millis(300))
        .to_have_css("color", "rgb(0, 255, 0)")
        .await;
    assert!(result.is_err(), "Wrong CSS value should time out");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_to_have_count_assertions() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<ul><li class='item'>A</li><li class='item'>B</li><li class='item'>C</li></ul>",
        None,
    )
    .await
    .expect("Failed to navigate");

    expect(page.locator(".item").await)
        .to_have_count(3)
        .await
        .expect("Should have 3 items");

    expect(page.locator(".missing").await)
        .to_have_count(0)
        .await
        .expect("Missing selector should have count 0");

    expect(page.locator(".item").await)
        .not()
        .to_have_count(5)
        .await
        .expect("Should NOT have count 5");

    // Auto-retry: list grows after a delay
    page.evaluate_expression(
        r#"
        setTimeout(() => {
            const li = document.createElement('li');
            li.className = 'item';
            li.textContent = 'D';
            document.querySelector('ul').appendChild(li);
        }, 150);
        "#,
    )
    .await
    .expect("Failed to inject script");

    expect(page.locator(".item").await)
        .to_have_count(4)
        .await
        .expect("Should auto-retry until count reaches 4");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
#[ignore]
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

// ============================================================================
// to_have_screenshot() Assertions
// ============================================================================

#[cfg(feature = "screenshot-diff")]
#[tokio::test]
async fn test_to_have_screenshot_creates_baseline() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(
        "<h1 style='color:red;font-family:monospace'>Hello Screenshot</h1>",
        None,
    )
    .await
    .expect("Failed to set content");

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let baseline_path = temp_dir.path().join("baseline.png");

    // First run: no baseline exists, should create it
    let locator = page.locator("h1").await;
    expect(locator)
        .to_have_screenshot(&baseline_path, None)
        .await
        .expect("First run should create baseline");

    assert!(baseline_path.exists(), "Baseline file should be created");
    let baseline_bytes = std::fs::read(&baseline_path).expect("Failed to read baseline");
    assert!(baseline_bytes.len() > 100, "Baseline should be a valid PNG");
    // PNG magic bytes
    assert_eq!(&baseline_bytes[..4], &[0x89, 0x50, 0x4E, 0x47]);
    tracing::info!("✓ Baseline created ({} bytes)", baseline_bytes.len());

    browser.close().await.expect("Failed to close browser");
}

#[cfg(feature = "screenshot-diff")]
#[tokio::test]
async fn test_to_have_screenshot_matches_baseline() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(
        "<div id='test' style='width:100px;height:100px;background:blue;font-family:monospace'>Test</div>",
        None,
    )
    .await
    .expect("Failed to set content");

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let baseline_path = temp_dir.path().join("match.png");

    let locator = page.locator("#test").await;

    // Create baseline
    expect(locator.clone())
        .to_have_screenshot(&baseline_path, None)
        .await
        .expect("Should create baseline");

    // Second run: same content should match
    expect(locator)
        .to_have_screenshot(&baseline_path, None)
        .await
        .expect("Second run should match baseline");

    tracing::info!("✓ Screenshot matches baseline");

    browser.close().await.expect("Failed to close browser");
}

#[cfg(feature = "screenshot-diff")]
#[tokio::test]
async fn test_to_have_screenshot_detects_difference() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Create baseline with blue background
    page.set_content(
        "<div id='test' style='width:100px;height:100px;background:blue;font-family:monospace'>Test</div>",
        None,
    )
    .await
    .expect("Failed to set content");

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let baseline_path = temp_dir.path().join("diff.png");

    let locator = page.locator("#test").await;
    expect(locator)
        .to_have_screenshot(&baseline_path, None)
        .await
        .expect("Should create baseline");

    // Change content to red
    page.set_content(
        "<div id='test' style='width:100px;height:100px;background:red;font-family:monospace'>Test</div>",
        None,
    )
    .await
    .expect("Failed to set content");

    let locator = page.locator("#test").await;
    let result = expect(locator)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_have_screenshot(&baseline_path, None)
        .await;

    assert!(result.is_err(), "Should detect screenshot difference");

    // Verify diff and actual images were saved
    let actual_path = temp_dir.path().join("diff-actual.png");
    let diff_path = temp_dir.path().join("diff-diff.png");
    assert!(actual_path.exists(), "Actual screenshot should be saved");
    assert!(diff_path.exists(), "Diff image should be saved");
    tracing::info!("✓ Screenshot difference detected, diff saved");

    browser.close().await.expect("Failed to close browser");
}

#[cfg(feature = "screenshot-diff")]
#[tokio::test]
async fn test_to_have_screenshot_max_diff_pixels() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Create baseline
    page.set_content(
        "<div id='test' style='width:50px;height:50px;background:blue;font-family:monospace'></div>",
        None,
    )
    .await
    .expect("Failed to set content");

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let baseline_path = temp_dir.path().join("tolerance.png");

    let locator = page.locator("#test").await;
    expect(locator)
        .to_have_screenshot(&baseline_path, None)
        .await
        .expect("Should create baseline");

    // Slightly different (add a small red border)
    page.evaluate_expression("document.querySelector('#test').style.borderTop = '1px solid red'")
        .await
        .expect("Failed to modify");

    // With generous tolerance, should pass
    let locator = page.locator("#test").await;
    let options = playwright_rs::ScreenshotAssertionOptions::builder()
        .max_diff_pixels(5000)
        .build();
    expect(locator)
        .to_have_screenshot(&baseline_path, Some(options))
        .await
        .expect("Should pass with max_diff_pixels tolerance");
    tracing::info!("✓ max_diff_pixels tolerance works");

    browser.close().await.expect("Failed to close browser");
}

#[cfg(feature = "screenshot-diff")]
#[tokio::test]
async fn test_to_have_screenshot_update_snapshots() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Create baseline with blue
    page.set_content(
        "<div id='test' style='width:50px;height:50px;background:blue;font-family:monospace'></div>",
        None,
    )
    .await
    .expect("Failed to set content");

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let baseline_path = temp_dir.path().join("update.png");

    let locator = page.locator("#test").await;
    expect(locator)
        .to_have_screenshot(&baseline_path, None)
        .await
        .expect("Should create baseline");

    // Change to red and update
    page.set_content(
        "<div id='test' style='width:50px;height:50px;background:red;font-family:monospace'></div>",
        None,
    )
    .await
    .expect("Failed to set content");

    let locator = page.locator("#test").await;
    let options = playwright_rs::ScreenshotAssertionOptions::builder()
        .update_snapshots(true)
        .build();
    expect(locator)
        .to_have_screenshot(&baseline_path, Some(options))
        .await
        .expect("Should update baseline");

    assert!(baseline_path.exists(), "Baseline should still exist");
    tracing::info!("✓ update_snapshots works");

    browser.close().await.expect("Failed to close browser");
}

#[cfg(feature = "screenshot-diff")]
#[tokio::test]
async fn test_to_have_screenshot_animations_disabled() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Page with CSS animation
    page.set_content(
        r#"<style>
            @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
            #spinner { width:50px; height:50px; background:green; animation: spin 1s infinite; font-family:monospace; }
        </style>
        <div id="spinner">Spin</div>"#,
        None,
    )
    .await
    .expect("Failed to set content");

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let baseline_path = temp_dir.path().join("animation.png");

    let locator = page.locator("#spinner").await;
    let options = playwright_rs::ScreenshotAssertionOptions::builder()
        .animations(playwright_rs::Animations::Disabled)
        .build();

    // Should create baseline with animations frozen
    expect(locator.clone())
        .to_have_screenshot(&baseline_path, Some(options.clone()))
        .await
        .expect("Should create baseline with animations disabled");

    // Second run should match (animations still disabled)
    expect(locator)
        .to_have_screenshot(&baseline_path, Some(options))
        .await
        .expect("Should match with animations disabled");

    tracing::info!("✓ animations: disabled works");

    browser.close().await.expect("Failed to close browser");
}

#[cfg(feature = "screenshot-diff")]
#[tokio::test]
async fn test_expect_page_to_have_screenshot() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(
        "<body style='margin:0;background:white;font-family:monospace'><h1>Page Screenshot</h1></body>",
        None,
    )
    .await
    .expect("Failed to set content");

    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let baseline_path = temp_dir.path().join("page.png");

    // Page-level screenshot assertion
    playwright_rs::expect_page(&page)
        .to_have_screenshot(&baseline_path, None)
        .await
        .expect("Should create page baseline");

    assert!(baseline_path.exists(), "Page baseline should be created");

    // Second run should match
    playwright_rs::expect_page(&page)
        .to_have_screenshot(&baseline_path, None)
        .await
        .expect("Page screenshot should match");

    tracing::info!("✓ expect_page().to_have_screenshot() works");

    browser.close().await.expect("Failed to close browser");
}
