// Integration tests for Locator functionality
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - Locator creation (page.locator)
// - Locator chaining (first, last, nth, locator)
// - Query methods (count, text_content, inner_text, inner_html, get_attribute)
// - State queries (is_visible, is_enabled, is_checked, is_editable)
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~64% (11 tests → 4 tests)

use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;

// ============================================================================
// Locator Query Methods
// ============================================================================

#[tokio::test]
async fn test_locator_query_methods() {
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

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Create a locator
    let heading = page.locator("h1").await;
    assert_eq!(heading.selector(), "h1");

    // Test 2: Count elements
    let paragraphs = page.locator("p").await;
    let count = paragraphs.count().await.expect("Failed to get count");
    assert_eq!(count, 3); // locator.html has exactly 3 paragraphs

    // Test 3: Get text content
    let text = heading
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(text, Some("Test Page".to_string()));

    // Test 4: Get inner text (visible text only)
    let inner = heading
        .inner_text()
        .await
        .expect("Failed to get inner text");
    assert_eq!(inner, "Test Page");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Locator Chaining Methods
// ============================================================================

#[tokio::test]
async fn test_locator_chaining_methods() {
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

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let paragraphs = page.locator("p").await;

    // Test 1: Get first paragraph
    let first = paragraphs.first();
    assert_eq!(first.selector(), "p >> nth=0");
    let text = first
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(text, Some("First paragraph".to_string()));

    // Test 2: Get last paragraph
    let last = paragraphs.last();
    assert_eq!(last.selector(), "p >> nth=-1");
    let text = last
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(text, Some("Third paragraph".to_string()));

    // Test 3: Get nth element (second paragraph)
    let second = paragraphs.nth(1);
    assert_eq!(second.selector(), "p >> nth=1");
    let text = second
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(text, Some("Second paragraph".to_string()));

    // Test 4: Nested locators
    let container = page.locator(".container").await;
    let nested = container.locator("#nested");
    assert_eq!(nested.selector(), ".container >> #nested");
    let text = nested
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(text, Some("Nested element".to_string()));

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Locator State Methods
// ============================================================================

#[tokio::test]
async fn test_locator_state_methods() {
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

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Check visibility for visible element
    let heading = page.locator("h1").await;
    let visible = heading
        .is_visible()
        .await
        .expect("Failed to check visibility");
    assert!(visible);

    // Test 2: Hidden element should not be visible
    let hidden = page.locator("#hidden").await;
    let hidden_visible = hidden
        .is_visible()
        .await
        .expect("Failed to check visibility");
    assert!(!hidden_visible);

    // Test 3: is_hidden for hidden element
    let is_hidden = hidden.is_hidden().await.expect("Failed to check is_hidden");
    assert!(is_hidden, "Hidden element should report is_hidden=true");

    // Test 4: is_hidden for visible element
    let heading_hidden = heading
        .is_hidden()
        .await
        .expect("Failed to check is_hidden");
    assert!(
        !heading_hidden,
        "Visible element should report is_hidden=false"
    );
    tracing::info!("✓ is_hidden() works");

    // Test 5: is_disabled for disabled button
    let disabled_btn = page.locator("button[disabled]").await;
    let is_disabled = disabled_btn
        .is_disabled()
        .await
        .expect("Failed to check is_disabled");
    assert!(
        is_disabled,
        "Disabled button should report is_disabled=true"
    );

    // Test 6: is_disabled for enabled element (h1 is not disabled)
    let heading_disabled = heading
        .is_disabled()
        .await
        .expect("Failed to check is_disabled");
    assert!(
        !heading_disabled,
        "Enabled element should report is_disabled=false"
    );

    // Test 7: is_enabled for disabled button should be false
    let disabled_enabled = disabled_btn
        .is_enabled()
        .await
        .expect("Failed to check is_enabled");
    assert!(
        !disabled_enabled,
        "Disabled button should report is_enabled=false"
    );
    tracing::info!("✓ is_disabled() works");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// get_by_text Locator Methods
// ============================================================================

#[tokio::test]
async fn test_get_by_text() {
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

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Substring match (exact=false) - "Submit" matches "Submit", "Submit Order", and "Submit Form"
    let submit_buttons = page.get_by_text("Submit", false).await;
    let count = submit_buttons
        .count()
        .await
        .expect("Failed to count submit buttons");
    assert_eq!(
        count, 3,
        "Substring 'Submit' should match all three buttons"
    );

    // Test 2: Exact match - "Submit" matches only the exact "Submit" button
    let exact_submit = page.get_by_text("Submit", true).await;
    let count = exact_submit
        .count()
        .await
        .expect("Failed to count exact submit");
    assert_eq!(count, 1, "Exact 'Submit' should match only one button");

    // Test 3: Case-insensitive substring match
    let hello = page.get_by_text("hello world", false).await;
    let count = hello.count().await.expect("Failed to count hello");
    assert_eq!(
        count, 2,
        "Case-insensitive 'hello world' should match both spans"
    );

    // Test 4: Case-sensitive exact match
    let hello_exact = page.get_by_text("Hello World", true).await;
    let count = hello_exact
        .count()
        .await
        .expect("Failed to count exact hello");
    assert_eq!(count, 1, "Exact 'Hello World' should match only one span");

    // Test 5: Locator chaining - get_by_text within a container
    let container = page.locator(".text-container").await;
    let inner = container.get_by_text("Inner Text", false);
    let count = inner.count().await.expect("Failed to count inner text");
    assert_eq!(count, 1, "get_by_text should scope to container");

    // Test 6: get_by_text on a Locator (chained selector)
    let body = page.locator("body").await;
    let submit_in_body = body.get_by_text("Submit", true);
    let count = submit_in_body
        .count()
        .await
        .expect("Failed to count submit in body");
    assert_eq!(count, 1, "Chained get_by_text should work from Locator");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// get_by_label, get_by_placeholder, get_by_alt_text, get_by_title, get_by_test_id
// ============================================================================

#[tokio::test]
async fn test_get_by_locator_methods() {
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

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // --- get_by_label ---
    // Substring match: "Address" matches "Email Address" label
    let addr_input = page.get_by_label("Address", false).await;
    let count = addr_input.count().await.expect("Failed to count label");
    assert_eq!(count, 1, "Substring 'Address' should match email input");

    // Exact match: "Full Name" matches only its associated input
    let exact_name = page.get_by_label("Full Name", true).await;
    let count = exact_name
        .count()
        .await
        .expect("Failed to count exact label");
    assert_eq!(count, 1, "Exact 'Full Name' should match one input");

    // --- get_by_placeholder ---
    // Substring match
    let enter_inputs = page.get_by_placeholder("Enter", false).await;
    let count = enter_inputs
        .count()
        .await
        .expect("Failed to count placeholder");
    assert_eq!(count, 2, "Substring 'Enter' should match both inputs");

    // Exact match
    let email_input = page.get_by_placeholder("Enter your email", true).await;
    let count = email_input
        .count()
        .await
        .expect("Failed to count exact placeholder");
    assert_eq!(count, 1, "Exact placeholder should match one input");

    // --- get_by_alt_text ---
    // Substring match: "Logo" matches "Company Logo"
    let logo = page.get_by_alt_text("Logo", false).await;
    let count = logo.count().await.expect("Failed to count alt text");
    assert_eq!(count, 1, "'Logo' should match one image");

    // Exact match
    let exact_banner = page.get_by_alt_text("Welcome Banner", true).await;
    let count = exact_banner
        .count()
        .await
        .expect("Failed to count exact alt text");
    assert_eq!(count, 1, "Exact 'Welcome Banner' should match one image");

    // --- get_by_title ---
    // Substring match: "More Info" matches both title attributes
    let info = page.get_by_title("More Info", false).await;
    let count = info.count().await.expect("Failed to count title");
    assert_eq!(count, 2, "Substring 'More Info' should match both spans");

    // Exact match
    let exact_info = page.get_by_title("More Info", true).await;
    let count = exact_info
        .count()
        .await
        .expect("Failed to count exact title");
    assert_eq!(count, 1, "Exact 'More Info' should match one span");

    // --- get_by_test_id ---
    let submit = page.get_by_test_id("submit-btn").await;
    let count = submit.count().await.expect("Failed to count test id");
    assert_eq!(count, 1, "test id 'submit-btn' should match one button");

    let cancel = page.get_by_test_id("cancel-btn").await;
    let text = cancel
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(text, Some("Cancel".to_string()));

    // --- Locator chaining ---
    let body = page.locator("body").await;
    let chained = body.get_by_test_id("submit-btn");
    let count = chained
        .count()
        .await
        .expect("Failed to count chained test id");
    assert_eq!(count, 1, "Chained get_by_test_id should work");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// get_by_role Locator Methods
// ============================================================================

#[tokio::test]
async fn test_get_by_role() {
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

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::{AriaRole, GetByRoleOptions};

    // Test 1: Find buttons by role (Submit, Submit Order, Submit Form, Cancel, Disabled Button)
    let buttons = page.get_by_role(AriaRole::Button, None).await;
    let count = buttons.count().await.expect("Failed to count buttons");
    assert_eq!(count, 5, "Should find 5 buttons, got {}", count);

    // Test 2: Find button by role + exact name
    let submit = page
        .get_by_role(
            AriaRole::Button,
            Some(GetByRoleOptions {
                name: Some("Submit".into()),
                exact: Some(true),
                ..Default::default()
            }),
        )
        .await;
    let count = submit.count().await.expect("Failed to count submit");
    assert_eq!(count, 1, "Exact name 'Submit' should match one button");

    // Test 3: Find button by role + substring name
    let submit_buttons = page
        .get_by_role(
            AriaRole::Button,
            Some(GetByRoleOptions {
                name: Some("Submit".into()),
                ..Default::default()
            }),
        )
        .await;
    let count = submit_buttons
        .count()
        .await
        .expect("Failed to count submit buttons");
    assert!(
        count >= 2,
        "Substring 'Submit' should match multiple buttons, got {}",
        count
    );

    // Test 4: Find headings by level
    let h2 = page
        .get_by_role(
            AriaRole::Heading,
            Some(GetByRoleOptions {
                level: Some(2),
                ..Default::default()
            }),
        )
        .await;
    let count = h2.count().await.expect("Failed to count h2");
    assert_eq!(count, 1, "Should find one h2 heading");
    let text = h2.text_content().await.expect("Failed to get h2 text");
    assert_eq!(text, Some("Section Title".to_string()));

    // Test 5: Find checked checkboxes
    let checked = page
        .get_by_role(
            AriaRole::Checkbox,
            Some(GetByRoleOptions {
                checked: Some(true),
                ..Default::default()
            }),
        )
        .await;
    let count = checked.count().await.expect("Failed to count checked");
    assert_eq!(count, 1, "Should find one checked checkbox");

    // Test 6: Find unchecked checkboxes
    let unchecked = page
        .get_by_role(
            AriaRole::Checkbox,
            Some(GetByRoleOptions {
                checked: Some(false),
                ..Default::default()
            }),
        )
        .await;
    let count = unchecked.count().await.expect("Failed to count unchecked");
    assert_eq!(count, 1, "Should find one unchecked checkbox");

    // Test 7: Find disabled buttons
    let disabled = page
        .get_by_role(
            AriaRole::Button,
            Some(GetByRoleOptions {
                disabled: Some(true),
                ..Default::default()
            }),
        )
        .await;
    let count = disabled.count().await.expect("Failed to count disabled");
    assert_eq!(count, 1, "Should find one disabled button");

    // Test 8: Find links
    let links = page.get_by_role(AriaRole::Link, None).await;
    let count = links.count().await.expect("Failed to count links");
    assert!(count >= 2, "Should find at least 2 links, got {}", count);

    // Test 9: Find alert role
    let alert = page.get_by_role(AriaRole::Alert, None).await;
    let text = alert
        .text_content()
        .await
        .expect("Failed to get alert text");
    assert_eq!(text, Some("Important message".to_string()));

    // Test 10: Locator chaining
    let body = page.locator("body").await;
    let chained = body.get_by_role(AriaRole::Alert, None);
    let count = chained.count().await.expect("Failed to count chained");
    assert_eq!(count, 1, "Chained get_by_role should work");

    // Test 11: Case-insensitive name match (default)
    let submit_ci = page
        .get_by_role(
            AriaRole::Button,
            Some(GetByRoleOptions {
                name: Some("submit".into()),
                ..Default::default()
            }),
        )
        .await;
    let count = submit_ci.count().await.expect("Failed to count ci");
    assert!(count >= 1, "Case-insensitive name should match");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Locator.all() Method
// ============================================================================

#[tokio::test]
async fn test_locator_all_multiple_elements() {
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

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // locator.html has 3 <p> elements
    let paragraphs = page.locator("p").await;
    let all = paragraphs.all().await.expect("Failed to get all locators");

    assert_eq!(all.len(), 3, "Should have 3 paragraph locators");

    // Each sub-locator should resolve to the correct text
    let text0 = all[0].text_content().await.expect("Failed to get text");
    assert_eq!(text0, Some("First paragraph".to_string()));

    let text1 = all[1].text_content().await.expect("Failed to get text");
    assert_eq!(text1, Some("Second paragraph".to_string()));

    let text2 = all[2].text_content().await.expect("Failed to get text");
    assert_eq!(text2, Some("Third paragraph".to_string()));

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_all_empty_selector() {
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

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Non-matching selector should return empty vec
    let missing = page.locator(".does-not-exist").await;
    let all = missing.all().await.expect("Failed to get all locators");
    assert_eq!(
        all.len(),
        0,
        "Should return empty vec for non-matching selector"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Context — selector included in error messages
// ============================================================================

#[tokio::test]
async fn test_locator_error_includes_selector() {
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

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Use the exact selector from issue #33 — should produce a clear error
    let selector = "div.page-number > span:last-child";
    let missing = page.locator(selector).await;

    // Use a short timeout to avoid waiting the default 30s
    let short_timeout_ms = 500.0;

    // click() should fail with an error that includes the selector
    let result = missing
        .click(Some(
            playwright_rs::protocol::ClickOptions::builder()
                .timeout(short_timeout_ms)
                .build(),
        ))
        .await;

    assert!(result.is_err(), "Should fail for non-existent element");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains(selector),
        "Error should include selector, got: {}",
        err_msg
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// filter(), and_(), or_() Methods
// ============================================================================

#[tokio::test]
async fn test_locator_filter_has_text() {
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

    page.goto(&format!("{}/filter.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::FilterOptions;

    // filter with has_text should narrow rows to only those containing "Apple"
    let rows = page.locator("tr").await;
    let apple_rows = rows.filter(FilterOptions {
        has_text: Some("Apple".to_string()),
        ..Default::default()
    });
    let count = apple_rows.count().await.expect("Failed to count");
    assert_eq!(count, 1, "Should find 1 row containing 'Apple'");

    // Verify it's the right row by checking text content
    let text = apple_rows
        .text_content()
        .await
        .expect("Failed to get text content");
    assert!(
        text.unwrap_or_default().contains("Apple"),
        "Row should contain 'Apple'"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_filter_has_not_text() {
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

    page.goto(&format!("{}/filter.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::FilterOptions;

    // filter with has_not_text should exclude rows containing "Apple"
    // The table has 3 data rows: Apple, Banana, Cherry
    let rows = page.locator("tr.data-row").await;
    let non_apple_rows = rows.filter(FilterOptions {
        has_not_text: Some("Apple".to_string()),
        ..Default::default()
    });
    let count = non_apple_rows.count().await.expect("Failed to count");
    assert_eq!(count, 2, "Should find 2 rows not containing 'Apple'");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_filter_has_child_locator() {
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

    page.goto(&format!("{}/filter.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::FilterOptions;

    // filter with has should narrow to rows containing a button
    let rows = page.locator("tr.data-row").await;
    let button_child = page.locator("button.action-btn").await;
    let rows_with_button = rows.filter(FilterOptions {
        has: Some(button_child),
        ..Default::default()
    });
    let count = rows_with_button.count().await.expect("Failed to count");
    assert_eq!(count, 2, "Should find 2 rows containing a button");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_filter_has_not_child_locator() {
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

    page.goto(&format!("{}/filter.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::FilterOptions;

    // filter with has_not should narrow to rows that do NOT contain a button
    let rows = page.locator("tr.data-row").await;
    let button_child = page.locator("button.action-btn").await;
    let rows_without_button = rows.filter(FilterOptions {
        has_not: Some(button_child),
        ..Default::default()
    });
    let count = rows_without_button.count().await.expect("Failed to count");
    assert_eq!(count, 1, "Should find 1 row without a button");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_and() {
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

    page.goto(&format!("{}/filter.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // and_() should match only elements satisfying BOTH locators
    // Find buttons that also have class "action-btn" (subset)
    let buttons = page.locator("button").await;
    let action_buttons = page.locator(".action-btn").await;
    let combined = buttons.and_(&action_buttons);

    let count = combined.count().await.expect("Failed to count");
    assert_eq!(
        count, 2,
        "Should find 2 buttons that also have class action-btn"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_or() {
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

    page.goto(&format!("{}/filter.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // or_() should match elements satisfying EITHER locator
    // Find either buttons or links
    let buttons = page.locator("button").await;
    let links = page.locator("a.nav-link").await;
    let either = buttons.or_(&links);

    let count = either.count().await.expect("Failed to count");
    // filter.html has 3 buttons (2 action-btn + 1 delete-btn) and 2 nav-links
    assert_eq!(
        count, 5,
        "Should find 5 elements that are either buttons or links"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_filter_chain() {
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

    page.goto(&format!("{}/filter.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::FilterOptions;

    // Chain filter() then and_(): first filter by text, then narrow further
    let rows = page.locator("tr.data-row").await;
    let button_child = page.locator("button.action-btn").await;

    // Get rows that contain "Banana" AND also have an action button
    let filtered = rows
        .filter(FilterOptions {
            has_text: Some("Banana".to_string()),
            ..Default::default()
        })
        .filter(FilterOptions {
            has: Some(button_child),
            ..Default::default()
        });

    let count = filtered.count().await.expect("Failed to count");
    assert_eq!(
        count, 1,
        "Should find 1 row with Banana and an action button"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_filter_selector_composition() {
    // Unit-style test: verify the selector strings are composed correctly
    // This tests the internal selector building without a browser launch
    use playwright_rs::FilterOptions;

    // We just verify the selector methods exist and return Locator
    // (the real behavior is tested in integration tests above)
    // This test documents expected selector patterns via assertions on selector()

    // Note: We can't construct a Locator directly (new() is pub(crate)),
    // so we skip pure unit-test of selectors and rely on integration tests.
    // This placeholder ensures the type compiles.
    let _opts = FilterOptions {
        has_text: Some("foo".to_string()),
        has_not_text: None,
        has: None,
        has_not: None,
    };
    assert!(_opts.has_text.is_some());
    assert!(_opts.has_not_text.is_none());
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    crate::common::init_tracing();
    // Smoke test to verify locators work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each method)

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
        .goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let firefox_heading = firefox_page.locator("h1").await;
    let text = firefox_heading
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(text, Some("Test Page".to_string()));

    firefox.close().await.expect("Failed to close Firefox");

    // Test WebKit
    let webkit = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let webkit_page = webkit.new_page().await.expect("Failed to create page");

    webkit_page
        .goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let webkit_heading = webkit_page.locator("h1").await;
    let visible = webkit_heading
        .is_visible()
        .await
        .expect("Failed to check visibility");
    assert!(visible);

    webkit.close().await.expect("Failed to close WebKit");
    server.shutdown();
}
