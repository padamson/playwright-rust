// Tests for BrowserContext.pages() and BrowserContext.browser() methods
//
// This test module verifies:
// - context.pages() returns empty vec initially
// - context.pages() includes pages after new_page()
// - context.pages() includes initial page in persistent context with --app
// - context.browser() returns Browser for regular contexts
// - context.browser() returns None for persistent contexts (Playwright behavior)
// - Cross-browser compatibility

use playwright_rs::protocol::Playwright;
use std::env;

#[tokio::test]
async fn test_context_pages_empty_initially() {
    let playwright = Playwright::launch().await.unwrap();
    let browser = playwright.chromium().launch().await.unwrap();
    let context = browser.new_context().await.unwrap();

    // Initially, context should have no pages
    let pages = context.pages();
    assert_eq!(pages.len(), 0, "New context should have no pages");

    context.close().await.unwrap();
    browser.close().await.unwrap();
}

#[tokio::test]
async fn test_context_pages_includes_new_page() {
    let playwright = Playwright::launch().await.unwrap();
    let browser = playwright.chromium().launch().await.unwrap();
    let context = browser.new_context().await.unwrap();

    // Create a page
    let page1 = context.new_page().await.unwrap();

    // Context should now include the page
    let pages = context.pages();
    assert_eq!(pages.len(), 1, "Context should have 1 page");

    // Verify it's the same page (by URL initially at about:blank)
    assert_eq!(pages[0].url(), page1.url());

    // Create another page
    let _page2 = context.new_page().await.unwrap();

    // Context should now include both pages
    let pages = context.pages();
    assert_eq!(pages.len(), 2, "Context should have 2 pages");

    context.close().await.unwrap();
    browser.close().await.unwrap();
}

#[tokio::test]
async fn test_context_pages_includes_initial_app_mode_page() {
    let playwright = Playwright::launch().await.unwrap();
    let chromium = playwright.chromium();

    // Create temp user data dir
    let user_data_dir = env::temp_dir()
        .join(format!(
            "pw-rust-test-app-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .to_string_lossy()
        .to_string();

    // Launch persistent context with --app mode
    let options = playwright_rs::protocol::BrowserContextOptions::builder()
        .headless(true)
        .args(vec!["--app=https://example.com".to_string()])
        .build();

    let context = chromium
        .launch_persistent_context_with_options(&user_data_dir, options)
        .await
        .unwrap();

    // In app mode, Playwright creates an initial page automatically
    let pages = context.pages();
    assert!(
        !pages.is_empty(),
        "Persistent context with --app should have initial page"
    );

    // The initial page should be navigating to or at the app URL
    // (may still be loading, so just check it exists)
    let initial_page = &pages[0];
    assert!(
        !initial_page.url().is_empty(),
        "Initial app mode page should have a URL"
    );

    context.close().await.unwrap();

    // Cleanup temp directory
    let _ = tokio::fs::remove_dir_all(&user_data_dir).await;
}

#[tokio::test]
async fn test_context_browser_returns_browser_for_regular_context() {
    let playwright = Playwright::launch().await.unwrap();
    let browser = playwright.chromium().launch().await.unwrap();
    let context = browser.new_context().await.unwrap();

    // Regular context should return the browser
    let context_browser = context.browser();
    assert!(
        context_browser.is_some(),
        "Regular context should return Some(Browser)"
    );

    // Verify it's the same browser by checking name and version
    let ctx_browser = context_browser.unwrap();
    assert_eq!(ctx_browser.name(), browser.name());
    assert_eq!(ctx_browser.version(), browser.version());

    context.close().await.unwrap();
    browser.close().await.unwrap();
}

#[tokio::test]
async fn test_context_browser_returns_browser_for_persistent_context() {
    let playwright = Playwright::launch().await.unwrap();
    let chromium = playwright.chromium();

    // Create temp user data dir
    let user_data_dir = env::temp_dir()
        .join(format!(
            "pw-rust-test-persist-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .to_string_lossy()
        .to_string();

    // Launch persistent context
    let options = playwright_rs::protocol::BrowserContextOptions::builder()
        .headless(true)
        .build();

    let context = chromium
        .launch_persistent_context_with_options(&user_data_dir, options)
        .await
        .unwrap();

    // Persistent context should ALSO return the browser (Playwright behavior)
    // Only Android/Electron contexts return None
    let context_browser = context.browser();
    assert!(
        context_browser.is_some(),
        "Persistent context should return Some(Browser)"
    );

    // Verify it's a valid browser
    let browser = context_browser.unwrap();
    assert_eq!(browser.name(), "chromium");
    assert!(!browser.version().is_empty());

    context.close().await.unwrap();

    // Cleanup temp directory
    let _ = tokio::fs::remove_dir_all(&user_data_dir).await;
}

#[tokio::test]
async fn test_context_pages_cross_browser() {
    let playwright = Playwright::launch().await.unwrap();

    // Test on all three browsers
    let browser_types = vec![
        ("chromium", playwright.chromium()),
        ("firefox", playwright.firefox()),
        ("webkit", playwright.webkit()),
    ];

    for (name, browser_type) in browser_types {
        println!("Testing context.pages() on {}", name);

        let browser = browser_type.launch().await.unwrap();
        let context = browser.new_context().await.unwrap();

        // Empty initially
        let pages = context.pages();
        assert_eq!(pages.len(), 0, "New {} context should have no pages", name);

        // Create page
        let _page = context.new_page().await.unwrap();

        // Should include page
        let pages = context.pages();
        assert_eq!(pages.len(), 1, "{} context should have 1 page", name);

        context.close().await.unwrap();
        browser.close().await.unwrap();
    }
}

#[tokio::test]
async fn test_context_browser_cross_browser() {
    let playwright = Playwright::launch().await.unwrap();

    // Test on all three browsers
    let browser_types = vec![
        ("chromium", playwright.chromium()),
        ("firefox", playwright.firefox()),
        ("webkit", playwright.webkit()),
    ];

    for (name, browser_type) in browser_types {
        println!("Testing context.browser() on {}", name);

        let browser = browser_type.launch().await.unwrap();
        let context = browser.new_context().await.unwrap();

        // Should return browser
        let context_browser = context.browser();
        assert!(
            context_browser.is_some(),
            "{} context should return Some(Browser)",
            name
        );

        let ctx_browser = context_browser.unwrap();
        assert_eq!(ctx_browser.name(), browser.name());
        assert_eq!(ctx_browser.version(), browser.version());

        context.close().await.unwrap();
        browser.close().await.unwrap();
    }
}
