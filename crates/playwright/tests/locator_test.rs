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

mod test_server;

use playwright_rs::protocol::Playwright;
use test_server::TestServer;

mod common;

// ============================================================================
// Locator Query Methods
// ============================================================================

#[tokio::test]
async fn test_locator_query_methods() {
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

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// get_by_text Locator Methods
// ============================================================================

#[tokio::test]
async fn test_get_by_text() {
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
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    common::init_tracing();
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
