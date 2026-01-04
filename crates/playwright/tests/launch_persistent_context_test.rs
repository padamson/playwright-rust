// Integration tests for BrowserType::launch_persistent_context()
//
// These tests verify persistent browser context launch with user data directory.
// Tests include: basic launch, app mode, storage persistence, and cross-browser compatibility.
//
// See: https://playwright.dev/docs/api/class-browsertype#browser-type-launch-persistent-context

use playwright_rs::protocol::{BrowserContextOptions, Playwright, Viewport};
use tempfile::TempDir;

mod common;

#[tokio::test]
async fn test_launch_persistent_context_basic() {
    common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_basic: Starting");

    // Create temporary directory for user data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    // Launch Playwright
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // Launch persistent context with basic options
    tracing::debug!("[TEST] Launching persistent context...");
    let context = chromium
        .launch_persistent_context(&user_data_dir)
        .await
        .expect("Failed to launch persistent context");

    // Verify context was created
    tracing::debug!("[TEST] Context created successfully");

    // Create a page and verify it works
    let page = context.new_page().await.expect("Failed to create page");
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Cleanup
    context.close().await.expect("Failed to close context");
    tracing::debug!("[TEST] test_launch_persistent_context_basic: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_with_options() {
    common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_with_options: Starting");

    // Create temporary directory for user data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // Create options with viewport and headless
    let options = BrowserContextOptions::builder()
        .viewport(Viewport {
            width: 1280,
            height: 720,
        })
        .build();

    // Launch persistent context with options
    let context = chromium
        .launch_persistent_context_with_options(&user_data_dir, options)
        .await
        .expect("Failed to launch persistent context with options");

    // Verify context works with options
    let page = context.new_page().await.expect("Failed to create page");
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Cleanup
    context.close().await.expect("Failed to close context");
    tracing::debug!("[TEST] test_launch_persistent_context_with_options: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_app_mode() {
    common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_app_mode: Starting");

    // Create temporary directory for user data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // Launch with app mode args
    let options = BrowserContextOptions::builder()
        .args(vec!["--app=https://example.com".to_string()])
        .headless(true) // App mode works in headless
        .build();

    let context = chromium
        .launch_persistent_context_with_options(&user_data_dir, options)
        .await
        .expect("Failed to launch persistent context in app mode");

    // Verify context was created
    // Note: In app mode, browser opens directly to the URL
    let _page = context.new_page().await.expect("Failed to create page");

    // Cleanup
    context.close().await.expect("Failed to close context");
    tracing::debug!("[TEST] test_launch_persistent_context_app_mode: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_storage_persistence() {
    common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_storage_persistence: Starting");

    // Create temporary directory for user data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // First session: set local storage
    {
        let context = chromium
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch first persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto("https://example.com", None)
            .await
            .expect("Failed to navigate");

        // Set local storage value
        page.evaluate_expression("localStorage.setItem('test_key', 'test_value')")
            .await
            .expect("Failed to set local storage");

        context.close().await.expect("Failed to close context");
    }

    // Second session: verify local storage persisted
    {
        let context = chromium
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch second persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto("https://example.com", None)
            .await
            .expect("Failed to navigate");

        // Retrieve local storage value
        let stored_value = page
            .evaluate_value("localStorage.getItem('test_key')")
            .await
            .expect("Failed to get local storage");

        assert_eq!(stored_value, "test_value", "Storage did not persist");

        context.close().await.expect("Failed to close context");
    }

    tracing::debug!("[TEST] test_launch_persistent_context_storage_persistence: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_error_handling() {
    common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_error_handling: Starting");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // Test with invalid user data directory (non-existent parent)
    let invalid_dir = "/nonexistent/path/to/userdata";

    let result = chromium.launch_persistent_context(invalid_dir).await;

    // Should return an error (though Playwright might create the directory)
    // This test mainly verifies the API accepts the parameter
    match result {
        Ok(context) => {
            // If it succeeds (Playwright created the directory), clean up
            let _ = context.close().await;
        }
        Err(_) => {
            // Error is acceptable
        }
    }

    tracing::debug!("[TEST] test_launch_persistent_context_error_handling: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_cross_browser() {
    common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_cross_browser: Starting");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test Chromium
    {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

        let chromium = playwright.chromium();
        let context = chromium
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch Chromium persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto("https://example.com", None)
            .await
            .expect("Failed to navigate in Chromium");

        context.close().await.expect("Failed to close Chromium");
        tracing::info!("✓ Chromium persistent context works");
    }

    // Test Firefox
    {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

        let firefox = playwright.firefox();
        let context = firefox
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch Firefox persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto("https://example.com", None)
            .await
            .expect("Failed to navigate in Firefox");

        context.close().await.expect("Failed to close Firefox");
        tracing::info!("✓ Firefox persistent context works");
    }

    // Test WebKit
    // Skip WebKit on Windows due to CI instability ("Initial load failed")
    if !cfg!(target_os = "windows") {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

        let webkit = playwright.webkit();
        let context = webkit
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch WebKit persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto("https://example.com", None)
            .await
            .expect("Failed to navigate in WebKit");

        context.close().await.expect("Failed to close WebKit");
        tracing::info!("✓ WebKit persistent context works");
    } else {
        tracing::warn!("Skipping WebKit persistent context test on Windows");
    }

    tracing::debug!("[TEST] test_launch_persistent_context_cross_browser: Complete");
}
