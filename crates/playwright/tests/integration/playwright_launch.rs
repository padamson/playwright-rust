use playwright_rs::protocol::Playwright;

#[tokio::test]
async fn test_playwright_launch() {
    crate::common::init_tracing();
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

    tracing::info!("✅ Playwright launched successfully!");
    tracing::info!("   Chromium: {}", chromium.executable_path());
    tracing::info!("   Firefox: {}", firefox.executable_path());
    tracing::info!("   WebKit: {}", webkit.executable_path());
}

/// Test that multiple Playwright instances can be created
///
/// Verifies that we can create multiple independent Playwright instances,
/// each with their own connection to separate server processes.
#[tokio::test]
async fn test_multiple_playwright_instances() {
    crate::common::init_tracing();
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
}

/// Test error handling: Error::ServerNotFound variant is defined and displays correctly.
#[tokio::test]
async fn test_launch_with_driver_not_found() {
    use playwright_rs::Error;

    let error = Error::ServerNotFound;
    let error_message = error.to_string();
    assert!(error_message.contains("Playwright server not found"));
}

/// Test graceful cleanup: dropping a Playwright instance allows creating a new one.
#[tokio::test]
async fn test_graceful_cleanup_on_drop() {
    crate::common::init_tracing();
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

    tracing::info!("✅ Graceful cleanup verified - can create new instance after drop!");
}

/// Test that playwright.devices() returns a non-empty map.
#[tokio::test]
async fn test_playwright_devices_not_empty() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let devices = playwright.devices();
    assert!(
        !devices.is_empty(),
        "devices() should return a non-empty map"
    );
}

/// Test that a known device like "iPhone 13" exists with expected fields.
#[tokio::test]
async fn test_playwright_devices_iphone() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let devices = playwright.devices();
    let iphone = devices.get("iPhone 13");
    assert!(iphone.is_some(), "devices() should contain 'iPhone 13'");

    let iphone = iphone.unwrap();
    assert!(
        !iphone.user_agent.is_empty(),
        "iPhone 13 user_agent should be non-empty"
    );
    assert!(
        iphone.viewport.width > 0,
        "iPhone 13 viewport width should be positive"
    );
    assert!(
        iphone.viewport.height > 0,
        "iPhone 13 viewport height should be positive"
    );
    assert!(
        iphone.device_scale_factor > 0.0,
        "iPhone 13 device_scale_factor should be positive"
    );
    assert!(iphone.is_mobile, "iPhone 13 should be mobile");
    assert!(iphone.has_touch, "iPhone 13 should have touch");
    assert_eq!(
        iphone.default_browser_type, "webkit",
        "iPhone 13 default browser should be webkit"
    );
}
