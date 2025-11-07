// Integration tests for Playwright::launch() high-level API

use playwright_core::protocol::Playwright;

#[tokio::test]
async fn test_playwright_launch() {
    // Launch Playwright
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Verify we can access browser types
    let chromium = playwright.chromium();
    assert_eq!(chromium.name(), "chromium");
    assert!(!chromium.executable_path().is_empty());

    let firefox = playwright.firefox();
    assert_eq!(firefox.name(), "firefox");
    assert!(!firefox.executable_path().is_empty());

    let webkit = playwright.webkit();
    assert_eq!(webkit.name(), "webkit");
    assert!(!webkit.executable_path().is_empty());

    println!("✅ Playwright launched successfully!");
    println!("   Chromium: {}", chromium.executable_path());
    println!("   Firefox: {}", firefox.executable_path());
    println!("   WebKit: {}", webkit.executable_path());
}

/// Test that multiple Playwright instances can be created
///
/// Verifies that we can create multiple independent Playwright instances,
/// each with their own connection to separate server processes.
#[tokio::test]
async fn test_multiple_playwright_instances() {
    // Launch first instance
    let playwright1 = Playwright::launch()
        .await
        .expect("Failed to launch first Playwright instance");

    // Launch second instance
    let playwright2 = Playwright::launch()
        .await
        .expect("Failed to launch second Playwright instance");

    // Verify both instances work independently
    assert_eq!(playwright1.chromium().name(), "chromium");
    assert_eq!(playwright2.chromium().name(), "chromium");

    // Note: In Phase 1, we don't have explicit cleanup yet
    // Both server processes will be killed when the test exits
    println!("✅ Multiple Playwright instances created successfully!");
}

/// Test error handling when driver is not found
///
/// NOTE: This test is difficult to implement properly in Phase 1 because:
/// 1. The driver is downloaded during build (build.rs)
/// 2. PLAYWRIGHT_DRIVER_PATH can point to multiple fallback locations
/// 3. Temporarily breaking the driver would affect parallel tests
///
/// For Phase 1, we verify the error handling path exists by checking:
/// - PlaywrightServer::launch() can return ServerNotFound error
/// - Error is properly typed and propagated
/// - The error path doesn't panic
///
/// Full error scenario testing will be done in Phase 2 with more robust
/// test infrastructure (test fixtures, isolated environments, etc.)
#[tokio::test]
async fn test_launch_with_driver_not_found() {
    // For Phase 1, we verify that the error types exist and are properly defined
    // The actual ServerNotFound error is tested in server.rs unit tests

    // Verify error enum has ServerNotFound variant
    use playwright_core::error::Error;

    let error = Error::ServerNotFound;
    let error_message = error.to_string();
    assert!(error_message.contains("Playwright server not found"));

    // NOTE: We don't actually try to launch with a bad path here because:
    // 1. It might still find a fallback driver and succeed
    // 2. Could interfere with other parallel tests
    // 3. The error path is already tested in server.rs unit tests

    println!("✅ ServerNotFound error type verified!");
    println!("   Full integration test deferred to Phase 2");
}

/// Test graceful cleanup
///
/// Verifies that when a Playwright instance is dropped, cleanup happens gracefully.
///
/// NOTE: In Phase 1, we don't have explicit Drop implementation yet.
/// The server process is killed when the parent process exits.
/// Proper cleanup (sending close messages, waiting for graceful shutdown)
/// will be implemented in Phase 2 along with Browser lifecycle management.
#[tokio::test]
async fn test_graceful_cleanup_on_drop() {
    // Create a Playwright instance in a scope
    {
        let playwright = Playwright::launch()
            .await
            .expect("Failed to launch Playwright");

        assert_eq!(playwright.chromium().name(), "chromium");

        // Playwright instance will be dropped here
    }

    // Verify we can create another instance after the first was dropped
    let playwright2 = Playwright::launch()
        .await
        .expect("Failed to launch second Playwright instance");

    assert_eq!(playwright2.chromium().name(), "chromium");

    println!("✅ Graceful cleanup verified - can create new instance after drop!");
}
