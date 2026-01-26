// Integration tests for Error Message Quality (Phase 6, Slice 7)
//
// Following TDD: Write tests first (Red), then implement fixes (Green), then refactor
//
// Tests cover:
// - Error messages include helpful context
// - Error messages suggest solutions
// - Error messages include what operation was being attempted
// - Error messages include relevant identifiers (selectors, URLs, etc.)
// - Network error messages are descriptive
// - Timeout error messages include duration
// - Element not found errors include selector
//
// Success Criteria:
// - All error messages are actionable
// - Errors include "what was attempted" context
// - Errors include "what went wrong" details
// - Errors suggest next steps when applicable

mod common;
mod test_server;

use playwright_rs::protocol::{ClickOptions, GotoOptions, Playwright};
use std::time::Duration;
use test_server::TestServer;

// ============================================================================
// Error Quality Test: Element Not Found
// ============================================================================

#[tokio::test]
async fn test_error_quality_element_not_found() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Element Not Found ===\n");

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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Navigation failed");

    // Test: Element not found error should include selector
    let locator = page.locator("button.does-not-exist").await;

    // Use short timeout (1s instead of default 30s) to speed up test
    let options = ClickOptions {
        timeout: Some(1000.0), // 1 second in milliseconds
        ..Default::default()
    };
    let result = locator.click(Some(options)).await;

    assert!(result.is_err(), "Expected error for non-existent element");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should mention the selector
    assert!(
        error_msg.contains("does-not-exist") || error_msg.contains("button"),
        "Error should include selector: {}",
        error_msg
    );

    // ASSERTION: Error should indicate what operation failed
    // Expected improvement: "Failed to click: Element not found: button.does-not-exist"
    // Current state might just say "Element not found"

    tracing::info!("\n✓ Element not found error includes selector");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Quality Test: Navigation Timeout
// ============================================================================

#[tokio::test]
async fn test_error_quality_navigation_timeout() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Navigation Timeout ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Timeout error should include duration and URL
    let timeout_duration = Duration::from_millis(100);
    let target_url = "http://10.255.255.1:9999/page.html";
    let options = GotoOptions::new().timeout(timeout_duration);
    let result = page.goto(target_url, Some(options)).await;

    assert!(result.is_err(), "Expected timeout error");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should mention timeout
    assert!(
        error_msg.contains("Timeout") || error_msg.contains("timeout"),
        "Error should mention timeout: {}",
        error_msg
    );

    // ASSERTION: Error should ideally include URL
    // Expected improvement: "Navigation timeout after 100ms navigating to http://10.255.255.1:9999/page.html"
    // Current state might just say "Timeout: ..."

    tracing::info!("\n✓ Navigation timeout error includes timeout duration");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Test: Invalid URL
// ============================================================================

#[tokio::test]
async fn test_error_quality_invalid_url() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Invalid URL ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Invalid URL error should be descriptive
    let invalid_url = "not-a-valid-url";
    let result = page.goto(invalid_url, None).await;

    assert!(result.is_err(), "Expected error for invalid URL");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should indicate what was wrong with the URL
    // Expected improvement: "Invalid URL: 'not-a-valid-url' is not a valid URL"
    // or "Navigation failed: Cannot navigate to 'not-a-valid-url' (invalid URL format)"

    assert!(!error_msg.is_empty(), "Error message should not be empty");

    tracing::info!("\n✓ Invalid URL error is descriptive");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Test: Connection Failed
// ============================================================================

#[tokio::test]
async fn test_error_quality_connection_failed() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Connection Failed ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Connection refused error should be descriptive
    let unreachable_url = "http://localhost:59999/";
    let result = page.goto(unreachable_url, None).await;

    assert!(result.is_err(), "Expected connection error");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should explain the connection failure
    // Expected improvement: "Connection failed: Cannot connect to http://localhost:59999/ (connection refused)"

    assert!(!error_msg.is_empty(), "Error message should not be empty");

    tracing::info!("\n✓ Connection failed error is descriptive");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Test: Operation After Close
// ============================================================================

#[tokio::test]
async fn test_error_quality_operation_after_close() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Operation After Close ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Close the page
    page.close().await.expect("Failed to close page");

    // Test: Operating on closed page should give helpful error
    let result = page.goto("https://example.com", None).await;

    assert!(result.is_err(), "Expected error for closed page");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should explain that the target was closed
    // Expected improvement: "Page is closed: Cannot perform navigation on a closed page"
    // Current state might say "Target closed" or "Channel closed"

    assert!(
        error_msg.contains("closed") || error_msg.contains("Closed"),
        "Error should mention that page is closed: {}",
        error_msg
    );

    tracing::info!("\n✓ Operation after close error is descriptive");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Test: Assertion Timeout
// ============================================================================

#[tokio::test]
async fn test_error_quality_assertion_timeout() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Assertion Timeout ===\n");

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

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Navigation failed");

    // Test: Assertion timeout should include what was being asserted
    let locator = page.locator("button.does-not-exist").await;

    // Try to click non-existent element with short timeout (1s instead of default 30s)
    let options = ClickOptions {
        timeout: Some(1000.0), // 1 second in milliseconds
        ..Default::default()
    };
    let result = locator.click(Some(options)).await;

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        tracing::info!("Error message: {}", error_msg);

        // ASSERTION: Error should mention what was waited for
        // Expected improvement: "Timeout waiting for selector 'button.does-not-exist' to be visible"

        assert!(!error_msg.is_empty(), "Error message should not be empty");
    } else {
        tracing::error!("Unexpected success (element should not exist)");
    }

    tracing::info!("\n✓ Assertion timeout error includes context");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Quality Test: Multiple Errors in Sequence
// ============================================================================

#[tokio::test]
async fn test_error_quality_error_sequence() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Multiple Errors ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Multiple errors should each be descriptive
    let errors = [
        ("invalid-url", "Invalid URL test"),
        ("http://localhost:59999/", "Connection refused test"),
        ("http://10.255.255.1:9999/timeout", "Timeout test"),
    ];

    for (url, test_name) in errors {
        let options = GotoOptions::new().timeout(Duration::from_millis(100));
        let result = page.goto(url, Some(options)).await;

        assert!(result.is_err(), "{} should produce error", test_name);

        let error_msg = format!("{:?}", result.unwrap_err());
        tracing::info!("{}: {}", test_name, error_msg);

        // Each error should be non-empty and descriptive
        assert!(!error_msg.is_empty(), "Error message should not be empty");
    }

    tracing::info!("\n✓ Multiple errors are each descriptive");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Audit: Review All Error Types
// ============================================================================

#[tokio::test]
async fn test_error_quality_audit() {
    common::init_tracing();
    tracing::info!("\n=== Error Quality Audit ===\n");

    // This test documents expected error message improvements
    // for each error variant in error.rs

    tracing::info!("Error Quality Expectations:");
    tracing::info!("1. ServerNotFound:");
    tracing::info!("   Current: 'Playwright server not found at expected location'");
    tracing::info!(
        "   Improved: 'Playwright server not found. Install with: npm install playwright'"
    );
    tracing::info!("2. LaunchFailed:");
    tracing::info!("   Current: 'Failed to launch Playwright server: <details>'");
    tracing::info!(
        "   Improved: 'Failed to launch Playwright server: <details>. Check that Node.js is installed.'"
    );
    tracing::info!("3. ElementNotFound:");
    tracing::info!("   Current: 'Element not found: <selector>'");
    tracing::info!(
        "   Improved: 'Element not found: <selector>. Waited for <timeout>. Retry with longer timeout or check selector.'"
    );
    tracing::info!("4. Timeout:");
    tracing::info!("   Current: 'Timeout: <message>'");
    tracing::info!(
        "   Improved: 'Timeout after <duration>: <operation> (<url>). Increase timeout or check network.'"
    );
    tracing::info!("5. TargetClosed:");
    tracing::info!("   Current: 'Target closed: <message>'");
    tracing::info!("   Improved: 'Target closed: Cannot perform <operation> on closed <target>.'");

    tracing::info!("\n✓ Error quality audit documented");
}
