// Integration tests for BrowserContext
//
// These tests verify that we can create browser contexts and manage them.

use playwright_rs::protocol::{BrowserContextOptions, Geolocation, Playwright, Viewport};
use std::env;

#[tokio::test]
async fn test_new_context() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();
    let browser = chromium.launch().await.expect("Failed to launch browser");

    // Create a new context
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Verify context was created
    tracing::info!("✓ Context created");

    // Cleanup
    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_multiple_contexts() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create multiple contexts
    let context1 = browser
        .new_context()
        .await
        .expect("Failed to create context 1");
    let context2 = browser
        .new_context()
        .await
        .expect("Failed to create context 2");

    tracing::info!("✓ Created 2 contexts");

    // Cleanup
    context1.close().await.expect("Failed to close context 1");
    context2.close().await.expect("Failed to close context 2");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Merged from: browser_context_options_test.rs
// ============================================================================

// BrowserContext Options Tests
//
// Tests for BrowserContext creation with various options (viewport, user agent, locale, etc.)
//
// These tests verify that:
// 1. Basic context options work (viewport, user agent, locale)
// 2. Geolocation options work correctly
// 3. Mobile emulation works (isMobile, hasTouch)
// 4. JavaScript disable/enable works
// 5. Offline mode works
// 6. Multiple options can be combined
//
// TDD approach: Tests written FIRST, then implementation

#[tokio::test]
async fn test_context_with_viewport() {
    crate::common::init_tracing();
    // Test creating context with custom viewport
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let options = BrowserContextOptions::builder()
        .viewport(Viewport {
            width: 1024,
            height: 768,
        })
        .build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with viewport");

    let page = context.new_page().await.expect("Failed to create page");

    // Verify viewport dimensions via JavaScript
    let width = page
        .evaluate_value("window.innerWidth")
        .await
        .expect("Failed to evaluate width");
    assert_eq!(width.parse::<i32>().unwrap(), 1024);

    let height = page
        .evaluate_value("window.innerHeight")
        .await
        .expect("Failed to evaluate height");
    assert_eq!(height.parse::<i32>().unwrap(), 768);

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_with_user_agent() {
    crate::common::init_tracing();
    // Test creating context with custom user agent
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let custom_ua = "CustomBot/1.0";
    let options = BrowserContextOptions::builder()
        .user_agent(custom_ua.to_string())
        .build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with user agent");

    let page = context.new_page().await.expect("Failed to create page");

    // Verify user agent via JavaScript
    let ua = page
        .evaluate_value("navigator.userAgent")
        .await
        .expect("Failed to evaluate user agent");
    assert_eq!(ua, custom_ua);

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_with_locale() {
    crate::common::init_tracing();
    // Test creating context with custom locale
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let options = BrowserContextOptions::builder()
        .locale("fr-FR".to_string())
        .build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with locale");

    let page = context.new_page().await.expect("Failed to create page");

    // Verify locale via JavaScript
    let locale = page
        .evaluate_value("navigator.language")
        .await
        .expect("Failed to evaluate locale");
    assert_eq!(locale, "fr-FR");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_with_timezone() {
    crate::common::init_tracing();
    // Test creating context with custom timezone
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let options = BrowserContextOptions::builder()
        .timezone_id("America/New_York".to_string())
        .build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with timezone");

    let page = context.new_page().await.expect("Failed to create page");

    // Verify timezone via JavaScript
    let tz = page
        .evaluate_value("Intl.DateTimeFormat().resolvedOptions().timeZone")
        .await
        .expect("Failed to evaluate timezone");
    assert_eq!(tz, "America/New_York");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_with_geolocation() {
    crate::common::init_tracing();
    // Test creating context with geolocation
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let options = BrowserContextOptions::builder()
        .geolocation(Geolocation {
            latitude: 48.8584, // Paris
            longitude: 2.2945,
            accuracy: Some(100.0),
        })
        .permissions(vec!["geolocation".to_string()])
        .build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with geolocation");

    let page = context.new_page().await.expect("Failed to create page");

    // Verify geolocation permission is granted by checking navigator.permissions
    // (actual geolocation requires page navigation which may be complex in tests)
    // Just verify the context was created successfully with geolocation options
    let has_geolocation = page
        .evaluate_value("'geolocation' in navigator")
        .await
        .expect("Failed to check geolocation");
    assert_eq!(has_geolocation, "true");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_offline_mode() {
    crate::common::init_tracing();
    // Test creating context in offline mode
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let options = BrowserContextOptions::builder().offline(true).build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context in offline mode");

    let page = context.new_page().await.expect("Failed to create page");

    // Try to navigate to a real website - should fail due to offline mode
    let result = page.goto("https://example.com", None).await;
    assert!(result.is_err(), "Navigation should fail in offline mode");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_combined_options() {
    crate::common::init_tracing();
    // Test creating context with multiple options combined
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let options = BrowserContextOptions::builder()
        .viewport(Viewport {
            width: 800,
            height: 600,
        })
        .user_agent("CustomBot/2.0".to_string())
        .locale("de-DE".to_string())
        .timezone_id("Europe/Berlin".to_string())
        .color_scheme("dark".to_string())
        .build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with combined options");

    let page = context.new_page().await.expect("Failed to create page");

    // Verify viewport
    let width = page
        .evaluate_value("window.innerWidth")
        .await
        .expect("Failed to evaluate width");
    assert_eq!(width.parse::<i32>().unwrap(), 800);

    // Verify user agent
    let ua = page
        .evaluate_value("navigator.userAgent")
        .await
        .expect("Failed to evaluate user agent");
    assert_eq!(ua, "CustomBot/2.0");

    // Verify locale
    let locale = page
        .evaluate_value("navigator.language")
        .await
        .expect("Failed to evaluate locale");
    assert_eq!(locale, "de-DE");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_no_viewport() {
    crate::common::init_tracing();
    // Test creating context with no viewport (null viewport)
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let options = BrowserContextOptions::builder().no_viewport(true).build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with no viewport");

    let page = context.new_page().await.expect("Failed to create page");

    // With no viewport, dimensions should match the browser window
    // This is typically larger than the default 1280x720
    let width = page
        .evaluate_value("window.innerWidth")
        .await
        .expect("Failed to evaluate width");

    // Just verify we got a reasonable width (should be different from default 1280)
    let width_val = width.parse::<i32>().unwrap();
    assert!(width_val > 0, "Width should be positive");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_cross_browser_options() {
    crate::common::init_tracing();
    // Verify context options work across Chromium, Firefox, and WebKit
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    for browser_name in &["chromium", "firefox", "webkit"] {
        let browser = match *browser_name {
            "chromium" => playwright.chromium().launch().await.unwrap(),
            "firefox" => playwright.firefox().launch().await.unwrap(),
            "webkit" => playwright.webkit().launch().await.unwrap(),
            _ => unreachable!(),
        };

        let options = BrowserContextOptions::builder()
            .viewport(Viewport {
                width: 640,
                height: 480,
            })
            .locale("en-US".to_string())
            .build();

        let context = browser
            .new_context_with_options(options)
            .await
            .unwrap_or_else(|e| panic!("Failed to create context in {}: {}", browser_name, e));

        let page = context
            .new_page()
            .await
            .unwrap_or_else(|e| panic!("Failed to create page in {}: {}", browser_name, e));

        // Verify viewport works in all browsers
        let width = page
            .evaluate_value("window.innerWidth")
            .await
            .unwrap_or_else(|e| panic!("Failed to evaluate in {}: {}", browser_name, e));
        assert_eq!(width.parse::<i32>().unwrap(), 640);

        context.close().await.unwrap();
        browser.close().await.unwrap();
    }
}

// ============================================================================
// StorageState Tests (Issue #6)
// ============================================================================

#[tokio::test]
async fn test_context_with_storage_state_inline() {
    use playwright_rs::protocol::{Cookie, LocalStorageItem, Origin, StorageState};
    crate::common::init_tracing();

    // Test creating context with inline storage state
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create storage state with cookies and localStorage
    let storage_state = StorageState {
        cookies: vec![Cookie {
            name: "test_cookie".to_string(),
            value: "test_value".to_string(),
            domain: ".example.com".to_string(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: Some("Lax".to_string()),
        }],
        origins: vec![Origin {
            origin: "https://example.com".to_string(),
            local_storage: vec![LocalStorageItem {
                name: "test_key".to_string(),
                value: "test_storage_value".to_string(),
            }],
        }],
    };

    let options = BrowserContextOptions::builder()
        .storage_state(storage_state)
        .build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with storage state");

    let page = context.new_page().await.expect("Failed to create page");

    // Navigate to example.com to verify storage state was loaded
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Verify cookie was set
    let cookie_value = page
        .evaluate_value("document.cookie")
        .await
        .expect("Failed to evaluate cookie");
    assert!(
        cookie_value.contains("test_cookie=test_value"),
        "Cookie should be set"
    );

    // Verify localStorage was set
    let storage_value = page
        .evaluate_value("localStorage.getItem('test_key')")
        .await
        .expect("Failed to evaluate localStorage");
    assert_eq!(storage_value, "test_storage_value");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_with_storage_state_from_file() {
    crate::common::init_tracing();

    // Test creating context with storage state from file
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create a temporary storage state file
    let temp_dir = std::env::temp_dir();
    let storage_file = temp_dir.join("test_storage_state.json");

    // Write storage state to file
    let storage_json = r#"{
        "cookies": [{
            "name": "file_cookie",
            "value": "file_value",
            "domain": ".example.com",
            "path": "/",
            "expires": -1,
            "httpOnly": false,
            "secure": false,
            "sameSite": "Lax"
        }],
        "origins": [{
            "origin": "https://example.com",
            "localStorage": [{
                "name": "file_key",
                "value": "file_storage_value"
            }]
        }]
    }"#;

    std::fs::write(&storage_file, storage_json).expect("Failed to write storage file");

    let options = BrowserContextOptions::builder()
        .storage_state_path(storage_file.to_str().unwrap().to_string())
        .build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with storage state from file");

    let page = context.new_page().await.expect("Failed to create page");

    // Navigate to example.com to verify storage state was loaded
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Verify cookie was set
    let cookie_value = page
        .evaluate_value("document.cookie")
        .await
        .expect("Failed to evaluate cookie");
    assert!(
        cookie_value.contains("file_cookie=file_value"),
        "Cookie from file should be set"
    );

    // Verify localStorage was set
    let storage_value = page
        .evaluate_value("localStorage.getItem('file_key')")
        .await
        .expect("Failed to evaluate localStorage");
    assert_eq!(storage_value, "file_storage_value");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");

    // Cleanup
    std::fs::remove_file(&storage_file).ok();
}

#[tokio::test]
async fn test_context_storage_state_invalid_file() {
    crate::common::init_tracing();

    // Test that invalid storage state file path returns error
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let options = BrowserContextOptions::builder()
        .storage_state_path("/nonexistent/path/to/storage.json".to_string())
        .build();

    let result = browser.new_context_with_options(options).await;

    // Should fail with error about missing file
    assert!(
        result.is_err(),
        "Creating context with non-existent storage file should fail"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_storage_state_cross_browser() {
    use playwright_rs::protocol::{Cookie, LocalStorageItem, Origin, StorageState};
    crate::common::init_tracing();

    // Verify storage state works across Chromium, Firefox, and WebKit
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    for browser_name in &["chromium", "firefox", "webkit"] {
        let browser = match *browser_name {
            "chromium" => playwright.chromium().launch().await.unwrap(),
            "firefox" => playwright.firefox().launch().await.unwrap(),
            "webkit" => playwright.webkit().launch().await.unwrap(),
            _ => unreachable!(),
        };

        // Create storage state with cookies
        let storage_state = StorageState {
            cookies: vec![Cookie {
                name: "browser_test_cookie".to_string(),
                value: format!("{}_value", browser_name),
                domain: ".example.com".to_string(),
                path: "/".to_string(),
                expires: -1.0,
                http_only: false,
                secure: false,
                same_site: Some("Lax".to_string()),
            }],
            origins: vec![Origin {
                origin: "https://example.com".to_string(),
                local_storage: vec![LocalStorageItem {
                    name: "browser_key".to_string(),
                    value: format!("{}_storage", browser_name),
                }],
            }],
        };

        let options = BrowserContextOptions::builder()
            .storage_state(storage_state)
            .build();

        let context = browser
            .new_context_with_options(options)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to create context with storage state in {}: {}",
                    browser_name, e
                )
            });

        let page = context
            .new_page()
            .await
            .unwrap_or_else(|e| panic!("Failed to create page in {}: {}", browser_name, e));

        // Navigate to example.com
        page.goto("https://example.com", None)
            .await
            .unwrap_or_else(|e| panic!("Failed to navigate in {}: {}", browser_name, e));

        // Verify cookie was set
        let cookie_value = page
            .evaluate_value("document.cookie")
            .await
            .unwrap_or_else(|e| panic!("Failed to evaluate cookie in {}: {}", browser_name, e));
        assert!(
            cookie_value.contains(&format!("browser_test_cookie={}_value", browser_name)),
            "Cookie should be set in {}",
            browser_name
        );

        context.close().await.unwrap();
        browser.close().await.unwrap();
    }
}

#[tokio::test]
async fn test_context_storage_state_empty() {
    use playwright_rs::protocol::StorageState;
    crate::common::init_tracing();

    // Test creating context with empty storage state (should work fine)
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let storage_state = StorageState {
        cookies: vec![],
        origins: vec![],
    };

    let options = BrowserContextOptions::builder()
        .storage_state(storage_state)
        .build();

    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context with empty storage state");

    let page = context.new_page().await.expect("Failed to create page");

    // Should work fine with no cookies/storage
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Merged from: browser_context_pages_test.rs
// ============================================================================

// Tests for BrowserContext.pages() and BrowserContext.browser() methods
//
// This test module verifies:
// - context.pages() returns empty vec initially
// - context.pages() includes pages after new_page()
// - context.pages() includes initial page in persistent context with --app
// - context.browser() returns Browser for regular contexts
// - context.browser() returns None for persistent contexts (Playwright behavior)
// - Cross-browser compatibility

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
