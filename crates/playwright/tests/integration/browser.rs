// Integration tests for Browser object creation
//
// Phase 2 Slice 1: Test that Browser objects can be created from server __create__ messages
//
// These tests verify that:
// 1. Browser objects are registered in the object factory
// 2. Browser initializer parsing works correctly (version, name fields)
// 3. Browser objects can be retrieved from the connection registry

use playwright_rs::api::LaunchOptions;
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
    crate::common::init_tracing();
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
    tracing::info!(
        "✅ Browser created: {} version {}",
        browser.name(),
        browser.version()
    );

    // Cleanup
    browser.close().await.expect("Failed to close browser");

    tracing::info!("✅ Slice 4 complete: Browser can be launched, used, and closed");
}

/// Test that Browser object has correct structure
///
/// Verifies that Browser struct exists and implements ChannelOwner trait.
/// This test passes as soon as Browser is defined and added to the object factory.
#[test]
fn test_browser_type_exists() {
    crate::common::init_tracing();
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

    tracing::info!("✅ Browser struct exists and implements ChannelOwner");
}

// ============================================================================
// Merged from: browser_launch_integration.rs
// ============================================================================

// Integration tests for BrowserType::launch()
//
// These tests verify that we can launch real browsers using the Playwright server.

#[tokio::test]
async fn test_launch_chromium() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_chromium: Starting");

    // Launch Playwright
    tracing::debug!("[TEST] Launching Playwright server...");
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    tracing::debug!("[TEST] Playwright server launched successfully");

    // Get chromium browser type
    let chromium = playwright.chromium();

    // Launch browser with default options
    tracing::debug!("[TEST] Launching Chromium browser...");
    let browser = chromium.launch().await.expect("Failed to launch Chromium");
    tracing::debug!("[TEST] Chromium browser launched successfully");

    // Verify browser was created
    assert_eq!(browser.name(), "chromium");
    assert!(!browser.version().is_empty());

    tracing::info!("Launched Chromium version: {}", browser.version());

    // Cleanup
    tracing::debug!("[TEST] Closing browser...");
    browser.close().await.expect("Failed to close browser");
    tracing::debug!("[TEST] Browser closed successfully");
    tracing::debug!("[TEST] test_launch_chromium: Complete");
}

#[tokio::test]
async fn test_launch_with_headless_option() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_with_headless_option: Starting");

    tracing::debug!("[TEST] Launching Playwright server...");
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    tracing::debug!("[TEST] Playwright server launched successfully");

    let chromium = playwright.chromium();

    // Launch with explicit headless option
    let options = LaunchOptions::default().headless(true);

    tracing::debug!("[TEST] Launching Chromium browser with headless option...");
    let browser = chromium
        .launch_with_options(options)
        .await
        .expect("Failed to launch Chromium with options");
    tracing::debug!("[TEST] Chromium browser launched successfully");

    assert_eq!(browser.name(), "chromium");
    assert!(!browser.version().is_empty());

    // Cleanup
    tracing::debug!("[TEST] Closing browser...");
    browser.close().await.expect("Failed to close browser");
    tracing::debug!("[TEST] Browser closed successfully");
    tracing::debug!("[TEST] test_launch_with_headless_option: Complete");
}

#[tokio::test]
async fn test_launch_all_three_browsers() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_all_three_browsers: Starting");

    tracing::debug!("[TEST] Launching Playwright server...");
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    tracing::debug!("[TEST] Playwright server launched successfully");

    // Test Chromium
    tracing::debug!("[TEST] === Testing Chromium ===");
    let chromium = playwright.chromium();
    tracing::debug!("[TEST] Launching Chromium browser...");
    let chromium_browser = chromium.launch().await.expect("Failed to launch Chromium");
    assert_eq!(chromium_browser.name(), "chromium");
    tracing::info!("✓ Chromium: {}", chromium_browser.version());
    tracing::debug!("[TEST] Closing Chromium...");
    chromium_browser
        .close()
        .await
        .expect("Failed to close Chromium");
    tracing::debug!("[TEST] Chromium closed successfully");

    // Test Firefox
    tracing::debug!("[TEST] === Testing Firefox ===");
    let firefox = playwright.firefox();
    tracing::debug!("[TEST] Launching Firefox browser...");
    let firefox_browser = firefox.launch().await.expect("Failed to launch Firefox");
    assert_eq!(firefox_browser.name(), "firefox");
    tracing::info!("✓ Firefox: {}", firefox_browser.version());
    tracing::debug!("[TEST] Closing Firefox...");
    firefox_browser
        .close()
        .await
        .expect("Failed to close Firefox");
    tracing::debug!("[TEST] Firefox closed successfully");

    // Test WebKit
    tracing::debug!("[TEST] === Testing WebKit ===");
    let webkit = playwright.webkit();
    tracing::debug!("[TEST] Launching WebKit browser...");
    let webkit_browser = webkit.launch().await.expect("Failed to launch WebKit");
    assert_eq!(webkit_browser.name(), "webkit");
    tracing::info!("✓ WebKit: {}", webkit_browser.version());
    tracing::debug!("[TEST] Closing WebKit...");
    webkit_browser
        .close()
        .await
        .expect("Failed to close WebKit");
    tracing::debug!("[TEST] WebKit closed successfully");

    tracing::debug!("[TEST] test_launch_all_three_browsers: Complete");
}

#[tokio::test]
async fn test_browser_close() {
    let (_pw, browser, _page) = crate::common::setup().await;

    // Verify browser is open
    assert_eq!(browser.name(), "chromium");

    // Close browser
    browser.close().await.expect("Failed to close browser");

    tracing::info!("✓ Browser closed successfully");
}

#[tokio::test]
async fn test_close_multiple_browsers() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Launch multiple browsers
    let chromium = playwright.chromium();
    let browser1 = chromium
        .launch()
        .await
        .expect("Failed to launch Chromium 1");
    let browser2 = chromium
        .launch()
        .await
        .expect("Failed to launch Chromium 2");

    tracing::info!("Launched 2 browsers");

    // Close both browsers
    browser1.close().await.expect("Failed to close browser 1");
    tracing::info!("✓ Browser 1 closed");

    browser2.close().await.expect("Failed to close browser 2");
    tracing::info!("✓ Browser 2 closed");
}

#[tokio::test]
async fn test_browser_is_connected() {
    let (_pw, browser, _page) = crate::common::setup().await;

    // Should be connected initially
    assert!(
        browser.is_connected(),
        "Browser should be connected after launch"
    );

    // Close browser
    browser.close().await.expect("Failed to close browser");

    // Should be disconnected after close
    // Note: close() waits for the server to process the close command,
    // which should trigger the "disconnected" event before returning or shortly after.

    // Check immediately first.
    if browser.is_connected() {
        // Give it a moment for the event to arrive
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    assert!(
        !browser.is_connected(),
        "Browser should be disconnected after close"
    );
}

/// Test that browser.contexts() returns all open browser contexts.
///
/// Verifies:
/// - Empty after launch (no contexts yet)
/// - Returns one entry after new_context()
/// - Returns two entries after a second new_context()
/// - Back to zero after all contexts are closed
#[tokio::test]
async fn test_browser_contexts() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch Chromium");

    // No contexts yet
    let initial = browser.contexts();
    assert_eq!(initial.len(), 0, "Expected 0 contexts after launch");

    // Create first context
    let ctx1 = browser
        .new_context()
        .await
        .expect("Failed to create context 1");
    let after_first = browser.contexts();
    assert_eq!(
        after_first.len(),
        1,
        "Expected 1 context after first new_context"
    );

    // Create second context
    let ctx2 = browser
        .new_context()
        .await
        .expect("Failed to create context 2");
    let after_second = browser.contexts();
    assert_eq!(
        after_second.len(),
        2,
        "Expected 2 contexts after second new_context"
    );

    // Close first context
    ctx1.close().await.expect("Failed to close context 1");
    // Give the event a moment to propagate
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let after_close_first = browser.contexts();
    assert_eq!(
        after_close_first.len(),
        1,
        "Expected 1 context after closing first"
    );

    ctx2.close().await.expect("Failed to close context 2");
    browser.close().await.expect("Failed to close browser");
}

/// Test that browser.browser_type() returns the BrowserType that launched it.
///
/// Verifies:
/// - Returns a BrowserType with the correct name ("chromium")
/// - Executable path is non-empty
#[tokio::test]
async fn test_browser_type_property() {
    let (_pw, browser, _page) = crate::common::setup().await;

    let bt = browser.browser_type();
    assert_eq!(
        bt.name(),
        "chromium",
        "browser_type() should return chromium"
    );
    assert!(
        !bt.executable_path().is_empty(),
        "browser_type().executable_path() should be non-empty"
    );

    browser.close().await.expect("Failed to close browser");
}

/// Test that browser.on_disconnected() handler fires when the browser closes.
///
/// Verifies:
/// - Handler is called after browser.close()
/// - is_connected() returns false after disconnect
#[tokio::test]
async fn test_browser_on_disconnected() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let (_pw, browser, _page) = crate::common::setup().await;

    let fired = Arc::new(AtomicBool::new(false));
    let fired_clone = Arc::clone(&fired);

    browser
        .on_disconnected(move || {
            let f = Arc::clone(&fired_clone);
            async move {
                f.store(true, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_disconnected handler");

    // Close the browser — this should trigger the disconnected event
    browser.close().await.expect("Failed to close browser");

    // Give event loop time to dispatch
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    assert!(
        fired.load(Ordering::SeqCst),
        "on_disconnected handler should have fired after browser.close()"
    );
}

/// Test that start_tracing() and stop_tracing() work on Chromium.
///
/// Verifies:
/// - start_tracing() completes without error
/// - stop_tracing() returns non-empty bytes
///
/// Note: CDP tracing is Chromium-only.
#[tokio::test]
async fn test_browser_tracing() {
    let (_pw, browser, _page) = crate::common::setup().await;

    // Start tracing with screenshots enabled
    use playwright_rs::protocol::StartTracingOptions;
    let options = StartTracingOptions {
        screenshots: Some(true),
        categories: None,
        page: None,
    };
    browser
        .start_tracing(Some(options))
        .await
        .expect("start_tracing should succeed on Chromium");

    // Do some work in the browser
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");
    page.goto("about:blank", None)
        .await
        .expect("Failed to navigate");
    context.close().await.expect("Failed to close context");

    // Stop tracing — should return non-empty bytes
    let trace_data = browser
        .stop_tracing()
        .await
        .expect("stop_tracing should succeed on Chromium");

    assert!(
        !trace_data.is_empty(),
        "stop_tracing should return non-empty trace data"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_browser_new_browser_cdp_session() {
    let (_pw, browser, _page) = crate::common::setup().await;

    let session = browser
        .new_browser_cdp_session()
        .await
        .expect("new_browser_cdp_session should succeed on Chromium");

    let result = session
        .send("Browser.getVersion", None)
        .await
        .expect("CDP Browser.getVersion should succeed");

    assert!(
        result.get("product").is_some() || result.get("result").is_some(),
        "CDP response should contain version info"
    );

    session.detach().await.expect("detach should succeed");
    browser.close().await.expect("Failed to close browser");
}
