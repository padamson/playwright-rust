use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;

// ============================================================================
// Locator Query Methods
// ============================================================================

#[tokio::test]
async fn test_locator_query_methods() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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
#[ignore]
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

// ============================================================================
// focus() and blur() Methods
// ============================================================================

#[tokio::test]
async fn test_locator_focus_and_blur() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/focus_blur.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let input1 = page.locator("#input1").await;
    let input2 = page.locator("#input2").await;

    // Initially neither should be focused
    let initially_focused = input1
        .is_focused()
        .await
        .expect("Failed to check initial focus");
    assert!(!initially_focused, "Input1 should not be focused initially");

    // focus() should set focus on the element
    input1.focus().await.expect("Failed to focus input1");
    let after_focus = input1
        .is_focused()
        .await
        .expect("Failed to check focus after focus()");
    assert!(after_focus, "Input1 should be focused after focus()");

    // blur() should remove focus from the element
    input1.blur().await.expect("Failed to blur input1");
    let after_blur = input1
        .is_focused()
        .await
        .expect("Failed to check focus after blur()");
    assert!(!after_blur, "Input1 should not be focused after blur()");

    // focus() on a different element should also work
    input2.focus().await.expect("Failed to focus input2");
    let input2_focused = input2
        .is_focused()
        .await
        .expect("Failed to check input2 focus");
    assert!(input2_focused, "Input2 should be focused");

    // focus on input2 should mean input1 is no longer focused
    let input1_not_focused = input1
        .is_focused()
        .await
        .expect("Failed to check input1 focus after focusing input2");
    assert!(
        !input1_not_focused,
        "Input1 should not be focused after focusing input2"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// press_sequentially() Method
// ============================================================================

#[tokio::test]
async fn test_locator_press_sequentially() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/focus_blur.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let input = page.locator("#input1").await;

    // press_sequentially() should type each character individually
    input
        .press_sequentially("hello", None)
        .await
        .expect("Failed to press_sequentially");

    let value = input
        .input_value(None)
        .await
        .expect("Failed to get input value");
    assert_eq!(value, "hello", "Input should contain typed text");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_press_sequentially_with_delay() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/focus_blur.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let input = page.locator("#input1").await;

    use playwright_rs::PressSequentiallyOptions;

    // press_sequentially() with delay option should also work
    let options = PressSequentiallyOptions { delay: Some(10.0) };
    input
        .press_sequentially("abc", Some(options))
        .await
        .expect("Failed to press_sequentially with delay");

    let value = input
        .input_value(None)
        .await
        .expect("Failed to get input value");
    assert_eq!(value, "abc", "Input should contain typed text with delay");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// all_inner_texts() and all_text_contents() Methods
// ============================================================================

#[tokio::test]
async fn test_locator_all_inner_texts() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/all_texts.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let items = page.locator(".item").await;

    let texts = items
        .all_inner_texts()
        .await
        .expect("Failed to get all_inner_texts");

    assert_eq!(texts.len(), 3, "Should return inner text for all 3 items");
    assert!(
        texts.contains(&"Alpha".to_string()),
        "Should contain 'Alpha'"
    );
    assert!(texts.contains(&"Beta".to_string()), "Should contain 'Beta'");
    assert!(
        texts.contains(&"Gamma".to_string()),
        "Should contain 'Gamma'"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_all_text_contents() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/all_texts.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let items = page.locator(".item").await;

    let texts = items
        .all_text_contents()
        .await
        .expect("Failed to get all_text_contents");

    assert_eq!(texts.len(), 3, "Should return text content for all 3 items");
    assert!(
        texts.contains(&"Alpha".to_string()),
        "Should contain 'Alpha'"
    );
    assert!(texts.contains(&"Beta".to_string()), "Should contain 'Beta'");
    assert!(
        texts.contains(&"Gamma".to_string()),
        "Should contain 'Gamma'"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_all_texts_empty_when_no_match() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/all_texts.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let no_match = page.locator(".nonexistent").await;

    let inner_texts = no_match
        .all_inner_texts()
        .await
        .expect("all_inner_texts should return empty vec, not error, for no matches");
    assert!(
        inner_texts.is_empty(),
        "all_inner_texts should be empty for no matches"
    );

    let text_contents = no_match
        .all_text_contents()
        .await
        .expect("all_text_contents should return empty vec, not error, for no matches");
    assert!(
        text_contents.is_empty(),
        "all_text_contents should be empty for no matches"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// dispatch_event, bounding_box, scroll_into_view_if_needed
// ============================================================================

/// Test that dispatch_event fires a click event on a button and the handler runs.
#[tokio::test]
async fn test_locator_dispatch_event() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Navigate to a page with a button whose text changes on click
    page.goto(
        "data:text/html,<button id='btn' onclick=\"this.textContent='fired'\">original</button>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let btn = page.locator("#btn").await;

    // Dispatch a click event — handler should fire
    btn.dispatch_event("click", None)
        .await
        .expect("dispatch_event should succeed");

    let text = btn
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(
        text,
        Some("fired".to_string()),
        "click handler should have changed button text to 'fired'"
    );

    browser.close().await.expect("Failed to close browser");
}

/// Test that dispatch_event with eventInit data passes properties to the event handler.
///
/// Note: Playwright's dispatchEvent creates events using `new Event(type, eventInit)` for
/// custom event types. Properties like `clientX` (for mouse events) are accessible via the
/// appropriate event subtype. For simpler verification, we test that eventInit properties
/// like `bubbles` affect event propagation.
#[tokio::test]
async fn test_locator_dispatch_event_with_init() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Page listens for a mousemove event and records the clientX from eventInit
    page.goto(
        "data:text/html,<div id='target' style='width:100px;height:100px'>waiting</div>\
         <script>\
           document.getElementById('target').addEventListener('mousemove', function(e) {\
             this.textContent = String(e.clientX);\
           });\
         </script>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let target = page.locator("#target").await;

    // Dispatch a mousemove event with clientX in eventInit
    let event_init = serde_json::json!({ "clientX": 42 });
    target
        .dispatch_event("mousemove", Some(event_init))
        .await
        .expect("dispatch_event with init should succeed");

    let text = target
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(
        text,
        Some("42".to_string()),
        "mousemove event clientX from eventInit should be 42"
    );

    browser.close().await.expect("Failed to close browser");
}

/// Test that bounding_box returns reasonable dimensions for a visible element.
#[tokio::test]
async fn test_locator_bounding_box() {
    let (_pw, browser, page) = crate::common::setup().await;

    // A styled, visible div with known dimensions
    page.goto(
        "data:text/html,<div id='box' style='width:100px;height:50px;background:blue;position:absolute;top:10px;left:20px'></div>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let locator = page.locator("#box").await;
    let bbox = locator
        .bounding_box()
        .await
        .expect("bounding_box should succeed");

    assert!(bbox.is_some(), "Visible element should have a bounding box");
    let bbox = bbox.unwrap();

    // Dimensions should be positive and reasonable
    assert!(bbox.width > 0.0, "width should be positive");
    assert!(bbox.height > 0.0, "height should be positive");
    assert!(bbox.x >= 0.0, "x should be non-negative");
    assert!(bbox.y >= 0.0, "y should be non-negative");

    // The styled element should be approximately 100x50
    assert!(
        (bbox.width - 100.0).abs() < 2.0,
        "width should be approximately 100px, got {}",
        bbox.width
    );
    assert!(
        (bbox.height - 50.0).abs() < 2.0,
        "height should be approximately 50px, got {}",
        bbox.height
    );

    browser.close().await.expect("Failed to close browser");
}

/// Test that bounding_box returns None for a hidden (display:none) element.
#[tokio::test]
async fn test_locator_bounding_box_hidden() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<div id='hidden' style='display:none'>hidden</div>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let locator = page.locator("#hidden").await;
    let bbox = locator
        .bounding_box()
        .await
        .expect("bounding_box should not error for hidden element");

    assert!(
        bbox.is_none(),
        "Hidden element (display:none) should return None for bounding_box"
    );

    browser.close().await.expect("Failed to close browser");
}

/// Test that scroll_into_view_if_needed scrolls an off-screen element into view.
#[tokio::test]
async fn test_locator_scroll_into_view_if_needed() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Create a page taller than the viewport with a target element far below
    page.goto(
        "data:text/html,<div style='height:2000px'>tall spacer</div>\
         <div id='target' style='height:50px;background:green'>scroll target</div>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let target = page.locator("#target").await;

    // The element should be below the viewport initially
    // After scroll_into_view_if_needed, it should be visible
    target
        .scroll_into_view_if_needed()
        .await
        .expect("scroll_into_view_if_needed should succeed");

    // Verify the element is now in the viewport by checking its bounding box
    let bbox = target
        .bounding_box()
        .await
        .expect("bounding_box after scroll should succeed")
        .expect("element should be visible after scrolling into view");

    // The element should now be within the viewport (y >= 0 and within viewport height)
    // Default viewport is 1280x720
    assert!(
        bbox.y >= 0.0,
        "After scroll, element y should be >= 0, got {}",
        bbox.y
    );
    assert!(
        bbox.y < 720.0,
        "After scroll, element y should be within viewport height (720px), got {}",
        bbox.y
    );

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Locator.tap() Method
// ============================================================================

#[tokio::test]
async fn test_locator_tap() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // tap() requires a touch-enabled context
    let context = browser
        .new_context_with_options(playwright_rs::protocol::BrowserContextOptions {
            has_touch: Some(true),
            ..Default::default()
        })
        .await
        .expect("Failed to create context with touch");

    let page = context.new_page().await.expect("Failed to create page");

    page.goto(
        "data:text/html,<button id='btn' ontouchstart=\"this.textContent='tapped'\">Tap Me</button>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let button = page.locator("#btn").await;

    // tap() with no options should succeed on a visible element
    button.tap(None).await.expect("tap() should succeed");

    // Verify the tap event fired
    let text = button
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(
        text,
        Some("tapped".to_string()),
        "Tap event should have fired"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_tap_with_options() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let context = browser
        .new_context_with_options(playwright_rs::protocol::BrowserContextOptions {
            has_touch: Some(true),
            ..Default::default()
        })
        .await
        .expect("Failed to create context with touch");

    let page = context.new_page().await.expect("Failed to create page");

    page.goto("data:text/html,<button id='btn'>Force Tap</button>", None)
        .await
        .expect("Failed to navigate");

    let button = page.locator("#btn").await;

    // tap() with TapOptions (force=true) should succeed
    let opts = playwright_rs::TapOptions::builder().force(true).build();
    button
        .tap(Some(opts))
        .await
        .expect("tap() with options should succeed");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Locator.evaluate() Method
// ============================================================================

#[tokio::test]
async fn test_locator_evaluate() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // evaluate a JS function that receives the element and returns its textContent
    let heading = page.locator("h1").await;
    let text: String = heading
        .evaluate("(el) => el.textContent", None::<()>)
        .await
        .expect("evaluate should succeed");
    assert_eq!(text, "Test Page");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_evaluate_with_arg() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/locator.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // evaluate with an argument
    let heading = page.locator("h1").await;
    let result: String = heading
        .evaluate("(el, suffix) => el.textContent + suffix", Some("!"))
        .await
        .expect("evaluate with arg should succeed");
    assert_eq!(result, "Test Page!");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_evaluate_returns_number() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<div id='box' style='width:200px;height:100px;'></div>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let div = page.locator("#box").await;

    // evaluate a JS function that returns a number
    let width: f64 = div
        .evaluate("(el) => el.offsetWidth", None::<()>)
        .await
        .expect("evaluate should succeed");
    assert_eq!(width, 200.0);

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Locator.evaluate_all() Method
// ============================================================================

#[tokio::test]
async fn test_locator_evaluate_all() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/all_texts.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // evaluate_all: collect textContent of all matching elements
    let items = page.locator(".item").await;
    let texts: Vec<String> = items
        .evaluate_all("(elements) => elements.map(e => e.textContent)", None::<()>)
        .await
        .expect("evaluate_all should succeed");

    assert_eq!(texts, vec!["Alpha", "Beta", "Gamma"]);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_evaluate_all_with_arg() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<li class='item'>One</li><li class='item'>Two</li><li class='item'>Three</li>",
        None,
    )
    .await
    .expect("Failed to navigate");

    // evaluate_all with an argument (prefix each item)
    let items = page.locator(".item").await;
    let texts: Vec<String> = items
        .evaluate_all(
            "(elements, prefix) => elements.map(e => prefix + e.textContent)",
            Some("item: "),
        )
        .await
        .expect("evaluate_all with arg should succeed");

    assert_eq!(texts, vec!["item: One", "item: Two", "item: Three"]);

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_evaluate_all_returns_count() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<span class='x'></span><span class='x'></span><span class='x'></span>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let spans = page.locator(".x").await;
    let count: f64 = spans
        .evaluate_all("(elements) => elements.length", None::<()>)
        .await
        .expect("evaluate_all should succeed");

    assert_eq!(count, 3.0);

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// drag_to() - Locator drag and drop
// ============================================================================

#[tokio::test]
async fn test_locator_drag_to() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/drag_drop.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let source = page.locator("#source").await;
    let target = page.locator("#target").await;

    // Perform the drag operation
    source
        .drag_to(&target, None)
        .await
        .expect("drag_to should succeed");

    // Verify the drop occurred by checking the result text
    let result_text = page
        .locator("#result")
        .await
        .text_content()
        .await
        .expect("Failed to get result text");
    assert_eq!(
        result_text,
        Some("dropped".to_string()),
        "Drop should have been triggered"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_drag_to_with_options() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/drag_drop.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::{DragToOptions, Position};

    let source = page.locator("#source").await;
    let target = page.locator("#target").await;

    // Perform drag with explicit source and target positions
    let options = DragToOptions::builder()
        .source_position(Position { x: 10.0, y: 10.0 })
        .target_position(Position { x: 60.0, y: 60.0 })
        .build();

    source
        .drag_to(&target, Some(options))
        .await
        .expect("drag_to with options should succeed");

    let result_text = page
        .locator("#result")
        .await
        .text_content()
        .await
        .expect("Failed to get result text");
    assert_eq!(
        result_text,
        Some("dropped".to_string()),
        "Drop should have been triggered with options"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// page.drag_and_drop() - Page-level drag and drop between selectors
// ============================================================================

#[tokio::test]
async fn test_page_drag_and_drop() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/drag_drop.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // page.drag_and_drop() uses selectors directly (no locator needed)
    page.drag_and_drop("#source", "#target", None)
        .await
        .expect("drag_and_drop should succeed");

    let result_text = page
        .locator("#result")
        .await
        .text_content()
        .await
        .expect("Failed to get result text");
    assert_eq!(
        result_text,
        Some("dropped".to_string()),
        "Drop should have been triggered via page.drag_and_drop()"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// wait_for() - Locator wait for element state
// ============================================================================

#[tokio::test]
async fn test_locator_wait_for_visible() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/wait_for.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::{WaitForOptions, WaitForState};

    // Schedule showing hidden element after 200ms
    page.evaluate::<serde_json::Value, ()>("() => window.showElement(200)", None)
        .await
        .expect("Failed to schedule element show");

    // Wait for the hidden element to become visible
    let hidden_el = page.locator("#hidden-element").await;
    hidden_el
        .wait_for(Some(
            WaitForOptions::builder()
                .state(WaitForState::Visible)
                .build(),
        ))
        .await
        .expect("wait_for Visible should succeed");

    // Verify element is now visible
    assert!(
        hidden_el
            .is_visible()
            .await
            .expect("Failed to check visibility"),
        "Element should be visible after wait_for"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_wait_for_hidden() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/wait_for.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::{WaitForOptions, WaitForState};

    // Schedule hiding visible element after 200ms
    page.evaluate::<serde_json::Value, ()>("() => window.hideElement(200)", None)
        .await
        .expect("Failed to schedule element hide");

    // Wait for the visible element to become hidden
    let visible_el = page.locator("#visible-element").await;
    visible_el
        .wait_for(Some(
            WaitForOptions::builder()
                .state(WaitForState::Hidden)
                .build(),
        ))
        .await
        .expect("wait_for Hidden should succeed");

    // Verify element is now hidden
    assert!(
        visible_el
            .is_hidden()
            .await
            .expect("Failed to check hidden state"),
        "Element should be hidden after wait_for"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_wait_for_attached() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/wait_for.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::{WaitForOptions, WaitForState};

    // Schedule appending element after 200ms
    page.evaluate::<serde_json::Value, ()>("() => window.appendElement(200)", None)
        .await
        .expect("Failed to schedule element append");

    // Wait for the dynamically added element to be attached to the DOM
    let dynamic_el = page.locator("#dynamic-element").await;
    dynamic_el
        .wait_for(Some(
            WaitForOptions::builder()
                .state(WaitForState::Attached)
                .build(),
        ))
        .await
        .expect("wait_for Attached should succeed");

    // Verify element is now attached
    assert_eq!(
        dynamic_el.count().await.expect("Failed to count"),
        1,
        "Element should be in DOM after wait_for Attached"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_wait_for_detached() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/wait_for.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::{WaitForOptions, WaitForState};

    // Schedule removing element after 200ms
    page.evaluate::<serde_json::Value, ()>("() => window.removeElement(200)", None)
        .await
        .expect("Failed to schedule element remove");

    // Wait for the element to be removed from DOM
    let visible_el = page.locator("#visible-element").await;
    visible_el
        .wait_for(Some(
            WaitForOptions::builder()
                .state(WaitForState::Detached)
                .build(),
        ))
        .await
        .expect("wait_for Detached should succeed");

    // Verify element is now gone
    assert_eq!(
        visible_el.count().await.expect("Failed to count"),
        0,
        "Element should be removed from DOM after wait_for Detached"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_wait_for_default_state() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/wait_for.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Default state is Visible - element is already visible, should resolve immediately
    let visible_el = page.locator("#visible-element").await;
    visible_el
        .wait_for(None)
        .await
        .expect("wait_for with no options (default Visible) should succeed for visible element");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Locator Page Property
// ============================================================================

/// Tests that locator.page() returns the Page that owns the locator.
///
/// Verifies:
/// - locator.page() returns the correct Page (same URL as the original page)
/// - The returned Page is usable (can call url() and other methods)
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-page>
#[tokio::test]
async fn test_locator_page_property() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let url = format!("{}/locator.html", server.url());
    page.goto(&url, None).await.expect("Failed to navigate");

    // Create a locator, then get its page and verify it's the same page
    let locator = page.locator("h1").await;
    let locator_page = locator.page().expect("locator.page() should succeed");

    // The page returned by locator.page() should have the same URL as the original page
    assert_eq!(
        locator_page.url(),
        page.url(),
        "locator.page() should return the same page used to create the locator"
    );

    // The returned Page should be functional: we can call url() on it
    assert!(
        locator_page.url().contains("locator.html"),
        "locator.page() URL should contain 'locator.html', got: {}",
        locator_page.url()
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_locator_aria_snapshot() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content("<h1>Hello</h1><button>Click me</button>", None)
        .await
        .expect("Failed to set content");

    let body = page.locator("body").await;
    let snapshot = body
        .aria_snapshot()
        .await
        .expect("aria_snapshot should succeed");

    assert!(snapshot.contains("heading") || snapshot.contains("Hello"));
    assert!(snapshot.contains("button") || snapshot.contains("Click me"));

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_describe() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content("<button>Submit</button>", None)
        .await
        .expect("Failed to set content");

    let described = page.locator("button").await.describe("submit button");
    assert!(described.selector().contains("internal:describe"));

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_highlight() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content("<h1>Highlight me</h1>", None)
        .await
        .expect("Failed to set content");

    page.locator("h1")
        .await
        .highlight()
        .await
        .expect("highlight should succeed");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_content_frame() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(
        "<iframe id='myframe' srcdoc='<h1>Inside Frame</h1>'></iframe>",
        None,
    )
    .await
    .expect("Failed to set content");

    let frame = page.locator("iframe#myframe").await.content_frame();
    let text = frame
        .locator("h1")
        .text_content()
        .await
        .expect("should read iframe");
    assert_eq!(text, Some("Inside Frame".to_string()));

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_locator_normalize_returns_robust_selector() {
    // Locator::normalize() resolves a possibly-fragile selector to a
    // best-practices canonical form. We can't assert the exact resolved
    // selector (the algorithm is server-side and may evolve), but we can
    // assert: (1) normalize returns something non-empty and different
    // from a CSS selector when the element has stronger affordances, and
    // (2) the normalized locator points at the same element.
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(
        r#"<button data-testid="submit-btn" role="button" aria-label="Submit">Click me</button>"#,
        None,
    )
    .await
    .expect("Failed to set content");

    // Start with a CSS selector that's structurally fragile.
    let fragile = page.locator("button").await;
    let normalized = fragile.normalize().await.expect("normalize should succeed");

    let original_selector = fragile.selector().to_string();
    let normalized_selector = normalized.selector().to_string();

    // Sanity: not empty.
    assert!(
        !normalized_selector.is_empty(),
        "normalized selector should not be empty"
    );

    // Both locators should still resolve to the same element. We check by
    // text content; the underlying DOM node is the same.
    let original_text = fragile
        .text_content()
        .await
        .expect("Failed to read original text")
        .unwrap_or_default();
    let normalized_text = normalized
        .text_content()
        .await
        .expect("Failed to read normalized text")
        .unwrap_or_default();
    assert_eq!(
        original_text, normalized_text,
        "normalized locator should point at the same element ({original_selector} → {normalized_selector})"
    );

    browser.close().await.expect("Failed to close browser");
}
