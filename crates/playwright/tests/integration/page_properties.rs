// Integration tests for Page/BrowserContext timeout infrastructure and properties:
//   - page.set_default_timeout()
//   - page.set_default_navigation_timeout()
//   - context.set_default_timeout()
//   - context.set_default_navigation_timeout()
//   - page.is_closed()
//   - page.frames()
//
// TDD: Tests written FIRST before any implementation.
// Red stage: these will not compile until the API is added.

use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;

// ============================================================================
// page.set_default_timeout()
// ============================================================================

/// Test that set_default_timeout causes action timeouts to happen faster.
///
/// We set a very short timeout (1 ms) then try to click a non-existent element.
/// This should fail with a timeout error much sooner than the default 30 s.
#[tokio::test]
async fn test_page_set_default_timeout() {
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

    // Set a 1 ms timeout — any action should time out almost immediately
    page.set_default_timeout(1.0).await;

    // Clicking a non-existent element must produce an error and do so quickly
    let start = std::time::Instant::now();
    let result = page
        .locator("#nonexistent-element-for-timeout-test")
        .await
        .click(None)
        .await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Expected timeout error, got Ok");
    // Should time out well within 5 seconds (the default is 30 s)
    assert!(
        elapsed.as_secs() < 5,
        "Timeout took too long ({:?}), set_default_timeout did not take effect",
        elapsed
    );

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// page.set_default_navigation_timeout()
// ============================================================================

/// Test that set_default_navigation_timeout causes navigation timeouts to fire faster.
///
/// We set a very short navigation timeout then navigate to a URL that will never
/// respond (a port with nothing listening).
#[tokio::test]
async fn test_page_set_default_navigation_timeout() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Set a 100 ms navigation timeout
    page.set_default_navigation_timeout(100.0).await;

    // Attempt to navigate to a non-routable IP; should time out quickly
    let start = std::time::Instant::now();
    let result = page.goto("http://10.255.255.1/timeout-test", None).await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Expected navigation timeout error, got Ok");
    // Should time out well within 5 seconds (the default is 30 s)
    assert!(
        elapsed.as_secs() < 5,
        "Navigation timeout took too long ({:?}), set_default_navigation_timeout did not take effect",
        elapsed
    );

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// context.set_default_timeout()
// ============================================================================

/// Test that context-level set_default_timeout propagates to pages in the context.
#[tokio::test]
async fn test_context_set_default_timeout() {
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
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Set a 1 ms timeout at the context level
    context.set_default_timeout(1.0).await;

    let page = context.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Clicking a non-existent element must fail with a timeout quickly
    let start = std::time::Instant::now();
    let result = page
        .locator("#nonexistent-context-timeout-element")
        .await
        .click(None)
        .await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Expected timeout error, got Ok");
    assert!(
        elapsed.as_secs() < 5,
        "Context timeout took too long ({:?}), set_default_timeout on context did not take effect",
        elapsed
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// context.set_default_navigation_timeout()
// ============================================================================

/// Test that context-level set_default_navigation_timeout causes navigation to fail fast.
#[tokio::test]
async fn test_context_set_default_navigation_timeout() {
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
        .new_context()
        .await
        .expect("Failed to create context");

    // Set a 100 ms navigation timeout at context level
    context.set_default_navigation_timeout(100.0).await;

    let page = context.new_page().await.expect("Failed to create page");

    let start = std::time::Instant::now();
    let result = page.goto("http://10.255.255.1/timeout-test", None).await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Expected navigation timeout error, got Ok");
    assert!(
        elapsed.as_secs() < 5,
        "Context navigation timeout took too long ({:?})",
        elapsed
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// page.is_closed()
// ============================================================================

/// Test that is_closed() returns false on an open page and true after close().
#[tokio::test]
async fn test_page_is_closed() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Page should not be closed initially
    assert!(!page.is_closed(), "Newly created page should not be closed");

    // Close the page
    page.close().await.expect("Failed to close page");

    // Page should now be closed
    assert!(page.is_closed(), "Page should be closed after close()");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// page.frames()
// ============================================================================

/// Test that frames() returns at least the main frame.
#[tokio::test]
async fn test_page_frames() {
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

    let url = format!("{}/", server.url());
    page.goto(&url, None).await.expect("Failed to navigate");

    // frames() must include at least the main frame
    let frames = page.frames().await.expect("Failed to get frames");
    assert!(
        !frames.is_empty(),
        "frames() must return at least one frame"
    );

    // The main frame URL should match the page URL (possibly with trailing slash)
    let main_frame = page.main_frame().await.expect("Failed to get main frame");
    let frame_url = main_frame.url();
    let page_url = page.url();

    assert!(
        frame_url == page_url || frame_url.trim_end_matches('/') == page_url.trim_end_matches('/'),
        "Main frame URL '{}' should match page URL '{}'",
        frame_url,
        page_url
    );

    // The first frame in frames() must be the main frame
    assert_eq!(
        frames[0].url(),
        main_frame.url(),
        "First frame in frames() must be the main frame"
    );

    browser.close().await.expect("Failed to close browser");
}
