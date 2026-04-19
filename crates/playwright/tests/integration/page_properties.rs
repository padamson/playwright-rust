use crate::test_server::TestServer;

// ============================================================================
// page.set_default_timeout() + page.frames() + page.is_closed()
// ============================================================================

/// Exercises set_default_timeout, frames(), main_frame(), and is_closed() in one session.
#[tokio::test]
async fn test_page_timeout_frames_and_is_closed() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // set_default_timeout: a 1 ms timeout causes action failures almost immediately
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    page.set_default_timeout(1.0).await;

    let start = std::time::Instant::now();
    let result = page
        .locator("#nonexistent-element-for-timeout-test")
        .await
        .click(None)
        .await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Expected timeout error, got Ok");
    assert!(
        elapsed.as_secs() < 5,
        "Timeout took too long ({:?}), set_default_timeout did not take effect",
        elapsed
    );

    // frames() returns at least the main frame; main frame URL matches page URL
    // Reset timeout to a reasonable value for the next navigation
    page.set_default_timeout(30_000.0).await;
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to re-navigate");

    let frames = page.frames().await.expect("Failed to get frames");
    assert!(
        !frames.is_empty(),
        "frames() must return at least one frame"
    );

    let main_frame = page.main_frame().await.expect("Failed to get main frame");
    let frame_url = main_frame.url();
    let page_url = page.url();

    assert!(
        frame_url == page_url || frame_url.trim_end_matches('/') == page_url.trim_end_matches('/'),
        "Main frame URL '{}' should match page URL '{}'",
        frame_url,
        page_url
    );

    assert_eq!(
        frames[0].url(),
        main_frame.url(),
        "First frame in frames() must be the main frame"
    );

    // is_closed() is false while page is open, true after close()
    assert!(!page.is_closed(), "Newly created page should not be closed");
    page.close().await.expect("Failed to close page");
    assert!(page.is_closed(), "Page should be closed after close()");

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
    let (_pw, browser, page) = crate::common::setup().await;

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
// context.set_default_timeout() + context.set_default_navigation_timeout()
// ============================================================================

/// Exercises context-level set_default_timeout and set_default_navigation_timeout.
#[tokio::test]
async fn test_context_timeout_methods() {
    let server = TestServer::start().await;
    let (_pw, browser, context) = crate::common::setup_context().await;

    // set_default_timeout: propagates to pages in the context
    context.set_default_timeout(1.0).await;

    let page = context.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

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

    page.close().await.expect("Failed to close page");

    // set_default_navigation_timeout: causes navigation to fail fast
    context.set_default_navigation_timeout(100.0).await;

    let page2 = context.new_page().await.expect("Failed to create page");

    let start = std::time::Instant::now();
    let result = page2.goto("http://10.255.255.1/timeout-test", None).await;
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
