// Integration tests for Browser object creation
//
// Phase 2 Slice 1: Test that Browser objects can be created from server __create__ messages
//
// These tests verify that:
// 1. Browser objects are registered in the object factory
// 2. Browser initializer parsing works correctly (version, name fields)
// 3. Browser objects can be retrieved from the connection registry

use playwright_rs::protocol::Playwright;

/// Test that a Browser object is created when the server sends a Browser __create__ message
///
/// Verifies the complete flow:
/// - Browser is added to object_factory.rs ✅
/// - Browser::new() parses initializer correctly ✅
/// - BrowserType::launch() is implemented (Slice 3) ✅
///
/// This test verifies the end-to-end Browser creation flow.
#[tokio::test]
async fn test_browser_object_creation_via_launch() {
    // Initialize Playwright
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Get chromium browser type
    let chromium = playwright.chromium();

    // Launch browser - this will:
    // 1. Send "launch" RPC to server
    // 2. Server creates Browser object
    // 3. Server sends __create__ message with Browser initializer
    // 4. Object factory creates Browser from initializer
    // 5. Browser added to connection registry
    // 6. launch() returns Browser reference

    let browser = chromium.launch().await.expect("Failed to launch browser");

    // Verify Browser object fields
    assert_eq!(browser.name(), "chromium");
    assert!(!browser.version().is_empty());
    println!(
        "✅ Browser created: {} version {}",
        browser.name(),
        browser.version()
    );

    // Cleanup
    browser.close().await.expect("Failed to close browser");

    println!("✅ Slice 4 complete: Browser can be launched, used, and closed");
}

/// Test that Browser object has correct structure
///
/// Verifies that Browser struct exists and implements ChannelOwner trait.
/// This test passes as soon as Browser is defined and added to the object factory.
#[test]
fn test_browser_type_exists() {
    // This is a compile-time test - if Browser doesn't exist or doesn't
    // implement ChannelOwner, this won't compile
    use playwright_rs::protocol::Browser;
    use playwright_rs::server::channel_owner::ChannelOwner;
    use std::any::TypeId;

    // Verify Browser type exists
    assert_eq!(TypeId::of::<Browser>(), TypeId::of::<Browser>());

    // Verify Browser implements ChannelOwner (dyn trait check)
    // This verifies the trait is implemented correctly
    fn assert_channel_owner<T: ChannelOwner + 'static>() {}
    assert_channel_owner::<Browser>();

    println!("✅ Browser struct exists and implements ChannelOwner");
}
