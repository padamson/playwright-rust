// Test for page.url() hash navigation behavior (Issue #26)
//
// Verifies that page.url() correctly reflects URL changes when navigating
// via anchor links (hash fragments).

use playwright_rs::protocol::Playwright;

mod common;
mod test_server;

/// Test that page.url() returns URL with hash after anchor navigation
#[tokio::test]
async fn test_url_includes_hash_after_anchor_click() {
    common::init_tracing();

    let server = test_server::TestServer::start().await;
    let base_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to test page with anchors
    let url = format!("{}/anchors.html", base_url);
    page.goto(&url, None).await.expect("Failed to navigate");

    // Verify initial URL (without hash)
    assert_eq!(page.url(), url);
    tracing::info!("Initial URL: {}", page.url());

    // Click anchor link to navigate to #section1
    let anchor = page.locator("#link-to-section1").await;
    anchor.click(None).await.expect("Failed to click anchor");

    // Wait a bit for navigation to settle
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Verify URL now includes the hash
    let expected_url = format!("{}#section1", url);
    let actual_url = page.url();
    tracing::info!("URL after anchor click: {}", actual_url);

    assert_eq!(
        actual_url, expected_url,
        "Expected URL '{}' but got '{}'",
        expected_url, actual_url
    );

    // Click another anchor to navigate to #section2
    let anchor2 = page.locator("#link-to-section2").await;
    anchor2
        .click(None)
        .await
        .expect("Failed to click second anchor");

    // Wait a bit for navigation to settle
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Verify URL updated to new hash
    let expected_url2 = format!("{}#section2", url);
    let actual_url2 = page.url();
    tracing::info!("URL after second anchor click: {}", actual_url2);

    assert_eq!(
        actual_url2, expected_url2,
        "Expected URL '{}' but got '{}'",
        expected_url2, actual_url2
    );

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that page.url() reflects JavaScript location.hash changes
#[tokio::test]
async fn test_url_includes_hash_after_js_navigation() {
    common::init_tracing();

    let server = test_server::TestServer::start().await;
    let base_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to test page
    let url = format!("{}/anchors.html", base_url);
    page.goto(&url, None).await.expect("Failed to navigate");

    // Verify initial URL (without hash)
    assert_eq!(page.url(), url);

    // Use JavaScript to change the hash
    page.evaluate_expression("window.location.hash = '#js-section'")
        .await
        .expect("Failed to execute JavaScript");

    // Wait a bit for navigation to settle
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Verify URL includes the hash set via JavaScript
    let expected_url = format!("{}#js-section", url);
    let actual_url = page.url();
    tracing::info!("URL after JS hash change: {}", actual_url);

    assert_eq!(
        actual_url, expected_url,
        "Expected URL '{}' but got '{}'",
        expected_url, actual_url
    );

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test cross-browser: Hash navigation works on Chromium, Firefox, and WebKit
#[tokio::test]
async fn test_url_hash_cross_browser() {
    common::init_tracing();

    let server = test_server::TestServer::start().await;
    let base_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test on all three browser engines
    for browser_name in ["chromium", "firefox", "webkit"] {
        tracing::info!("Testing hash navigation on {}", browser_name);

        let browser = match browser_name {
            "chromium" => playwright.chromium().launch().await,
            "firefox" => playwright.firefox().launch().await,
            "webkit" => playwright.webkit().launch().await,
            _ => unreachable!(),
        }
        .unwrap_or_else(|_| panic!("Failed to launch {}", browser_name));

        let page = browser.new_page().await.expect("Failed to create page");

        // Navigate to test page
        let url = format!("{}/anchors.html", base_url);
        page.goto(&url, None).await.expect("Failed to navigate");

        // Click anchor link
        let anchor = page.locator("#link-to-section1").await;
        anchor.click(None).await.expect("Failed to click anchor");

        // Wait for navigation
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify URL includes hash on this browser
        let expected_url = format!("{}#section1", url);
        let actual_url = page.url();

        assert_eq!(
            actual_url, expected_url,
            "Hash navigation failed on {}: expected '{}' but got '{}'",
            browser_name, expected_url, actual_url
        );

        tracing::info!("✓ {} hash navigation works", browser_name);

        // Cleanup
        page.close().await.expect("Failed to close page");
        browser.close().await.expect("Failed to close browser");
    }

    server.shutdown();
}

/// Test that workaround using evaluate_value still works
#[tokio::test]
async fn test_url_workaround_with_evaluate() {
    common::init_tracing();

    let server = test_server::TestServer::start().await;
    let base_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to test page
    let url = format!("{}/anchors.html", base_url);
    page.goto(&url, None).await.expect("Failed to navigate");

    // Click anchor link
    let anchor = page.locator("#link-to-section1").await;
    anchor.click(None).await.expect("Failed to click anchor");

    // Wait for navigation
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Workaround: Use evaluate_value to get current URL
    let url_via_evaluate = page
        .evaluate_value("window.location.href")
        .await
        .expect("Failed to evaluate window.location.href");

    let expected_url = format!("{}#section1", url);

    // The workaround should always work
    assert_eq!(
        url_via_evaluate, expected_url,
        "Workaround failed: expected '{}' but got '{}'",
        expected_url, url_via_evaluate
    );

    tracing::info!("✓ Workaround using evaluate_value works");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
