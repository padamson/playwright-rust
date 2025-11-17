// Integration tests for BrowserType::launch()
//
// These tests verify that we can launch real browsers using the Playwright server.

use playwright_rs::api::LaunchOptions;
use playwright_rs::protocol::Playwright;

#[tokio::test]
async fn test_launch_chromium() {
    eprintln!("[TEST] test_launch_chromium: Starting");

    // Launch Playwright
    eprintln!("[TEST] Launching Playwright server...");
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    eprintln!("[TEST] Playwright server launched successfully");

    // Get chromium browser type
    let chromium = playwright.chromium();

    // Launch browser with default options
    eprintln!("[TEST] Launching Chromium browser...");
    let browser = chromium.launch().await.expect("Failed to launch Chromium");
    eprintln!("[TEST] Chromium browser launched successfully");

    // Verify browser was created
    assert_eq!(browser.name(), "chromium");
    assert!(!browser.version().is_empty());

    println!("Launched Chromium version: {}", browser.version());

    // Cleanup
    eprintln!("[TEST] Closing browser...");
    browser.close().await.expect("Failed to close browser");
    eprintln!("[TEST] Browser closed successfully");
    eprintln!("[TEST] test_launch_chromium: Complete");
}

#[tokio::test]
async fn test_launch_with_headless_option() {
    eprintln!("[TEST] test_launch_with_headless_option: Starting");

    eprintln!("[TEST] Launching Playwright server...");
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    eprintln!("[TEST] Playwright server launched successfully");

    let chromium = playwright.chromium();

    // Launch with explicit headless option
    let options = LaunchOptions::default().headless(true);

    eprintln!("[TEST] Launching Chromium browser with headless option...");
    let browser = chromium
        .launch_with_options(options)
        .await
        .expect("Failed to launch Chromium with options");
    eprintln!("[TEST] Chromium browser launched successfully");

    assert_eq!(browser.name(), "chromium");
    assert!(!browser.version().is_empty());

    // Cleanup
    eprintln!("[TEST] Closing browser...");
    browser.close().await.expect("Failed to close browser");
    eprintln!("[TEST] Browser closed successfully");
    eprintln!("[TEST] test_launch_with_headless_option: Complete");
}

#[tokio::test]
async fn test_launch_all_three_browsers() {
    eprintln!("[TEST] test_launch_all_three_browsers: Starting");

    eprintln!("[TEST] Launching Playwright server...");
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    eprintln!("[TEST] Playwright server launched successfully");

    // Test Chromium
    eprintln!("[TEST] === Testing Chromium ===");
    let chromium = playwright.chromium();
    eprintln!("[TEST] Launching Chromium browser...");
    let chromium_browser = chromium.launch().await.expect("Failed to launch Chromium");
    assert_eq!(chromium_browser.name(), "chromium");
    println!("✓ Chromium: {}", chromium_browser.version());
    eprintln!("[TEST] Closing Chromium...");
    chromium_browser
        .close()
        .await
        .expect("Failed to close Chromium");
    eprintln!("[TEST] Chromium closed successfully");

    // Test Firefox
    eprintln!("[TEST] === Testing Firefox ===");
    let firefox = playwright.firefox();
    eprintln!("[TEST] Launching Firefox browser...");
    let firefox_browser = firefox.launch().await.expect("Failed to launch Firefox");
    assert_eq!(firefox_browser.name(), "firefox");
    println!("✓ Firefox: {}", firefox_browser.version());
    eprintln!("[TEST] Closing Firefox...");
    firefox_browser
        .close()
        .await
        .expect("Failed to close Firefox");
    eprintln!("[TEST] Firefox closed successfully");

    // Test WebKit
    eprintln!("[TEST] === Testing WebKit ===");
    let webkit = playwright.webkit();
    eprintln!("[TEST] Launching WebKit browser...");
    let webkit_browser = webkit.launch().await.expect("Failed to launch WebKit");
    assert_eq!(webkit_browser.name(), "webkit");
    println!("✓ WebKit: {}", webkit_browser.version());
    eprintln!("[TEST] Closing WebKit...");
    webkit_browser
        .close()
        .await
        .expect("Failed to close WebKit");
    eprintln!("[TEST] WebKit closed successfully");

    eprintln!("[TEST] test_launch_all_three_browsers: Complete");
}

#[tokio::test]
async fn test_browser_close() {
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();
    let browser = chromium.launch().await.expect("Failed to launch Chromium");

    // Verify browser is open
    assert_eq!(browser.name(), "chromium");

    // Close browser
    browser.close().await.expect("Failed to close browser");

    println!("✓ Browser closed successfully");
}

#[tokio::test]
async fn test_close_multiple_browsers() {
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

    println!("Launched 2 browsers");

    // Close both browsers
    browser1.close().await.expect("Failed to close browser 1");
    println!("✓ Browser 1 closed");

    browser2.close().await.expect("Failed to close browser 2");
    println!("✓ Browser 2 closed");
}
