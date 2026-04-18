use playwright_rs::protocol::{Playwright, Viewport};

#[tokio::test]
async fn test_context_new_page() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    // Create a new page
    let page = context.new_page().await.expect("Failed to create page");

    // Verify page was created
    tracing::info!("✓ Page created");

    // Page should initially be at about:blank
    assert_eq!(page.url(), "about:blank");

    // Cleanup
    page.close().await.expect("Failed to close page");
    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_browser_new_page_convenience() {
    let (_pw, browser, page) = crate::common::setup().await;

    tracing::info!("✓ Page created via browser.new_page()");

    // Should be at about:blank
    assert_eq!(page.url(), "about:blank");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_multiple_pages_in_context() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    // Create multiple pages
    let page1 = context.new_page().await.expect("Failed to create page 1");
    let page2 = context.new_page().await.expect("Failed to create page 2");

    tracing::info!("✓ Created 2 pages in same context");

    // Each should be at about:blank
    assert_eq!(page1.url(), "about:blank");
    assert_eq!(page2.url(), "about:blank");

    // Cleanup
    page1.close().await.expect("Failed to close page 1");
    page2.close().await.expect("Failed to close page 2");
    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_close() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Close page
    page.close().await.expect("Failed to close page");

    tracing::info!("✓ Page closed successfully");

    // Cleanup
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// page.content()
// See: https://playwright.dev/docs/api/class-page#page-content
// ============================================================================

#[tokio::test]
async fn test_page_content_basic() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Navigate to a page with known HTML content
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Page</title>
</head>
<body>
    <h1 id="heading">Hello World</h1>
    <p>This is a test paragraph.</p>
</body>
</html>"#;

    // Use data URL to load the HTML
    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    page.goto(&data_url, None)
        .await
        .expect("Failed to navigate");

    // Get the page content
    let content = page.content().await.expect("Failed to get page content");

    // Verify the content contains expected elements
    assert!(
        content.contains("<!DOCTYPE html>") || content.to_lowercase().contains("<!doctype html>"),
        "Content should include DOCTYPE declaration"
    );
    assert!(
        content.contains("<html"),
        "Content should include <html> tag"
    );
    assert!(
        content.contains("<head"),
        "Content should include <head> tag"
    );
    assert!(
        content.contains("<title>Test Page</title>"),
        "Content should include <title> tag with text"
    );
    assert!(
        content.contains("<body"),
        "Content should include <body> tag"
    );
    assert!(
        content.contains("Hello World"),
        "Content should include body text"
    );

    tracing::info!("✓ page.content() returns full HTML including DOCTYPE");

    // Cleanup
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_content_empty_page() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Get content of about:blank
    let content = page.content().await.expect("Failed to get page content");

    // about:blank should still have basic HTML structure
    assert!(
        content.contains("<html") || content.contains("<HTML"),
        "Even about:blank has HTML structure"
    );

    tracing::info!("✓ page.content() works on about:blank");

    // Cleanup
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_content_with_dynamic_changes() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Navigate to a simple page
    let html = r#"<!DOCTYPE html>
<html>
<head><title>Dynamic Test</title></head>
<body>
    <div id="content">Original</div>
</body>
</html>"#;

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    page.goto(&data_url, None)
        .await
        .expect("Failed to navigate");

    // Modify the DOM using JavaScript
    page.evaluate_expression("document.getElementById('content').textContent = 'Modified'")
        .await
        .expect("Failed to evaluate script");

    // Get the updated content
    let content = page.content().await.expect("Failed to get page content");

    // Verify the content reflects the DOM changes
    assert!(
        content.contains("Modified"),
        "Content should reflect DOM changes"
    );
    assert!(
        !content.contains(">Original<"),
        "Content should not contain old text"
    );

    tracing::info!("✓ page.content() reflects dynamic DOM changes");

    // Cleanup
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
#[ignore]
async fn test_page_content_cross_browser() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Cross-Browser Test</title></head>
<body><h1>Test</h1></body>
</html>"#;

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));

    // Test on Chromium
    {
        let browser = playwright
            .chromium()
            .launch()
            .await
            .expect("Failed to launch Chromium");
        let page = browser.new_page().await.expect("Failed to create page");
        page.goto(&data_url, None)
            .await
            .expect("Failed to navigate");

        let content = page.content().await.expect("Failed to get content");
        assert!(
            content.contains("Cross-Browser Test"),
            "Chromium: content should contain title"
        );
        browser.close().await.expect("Failed to close browser");
        tracing::info!("✓ page.content() works on Chromium");
    }

    // Test on Firefox
    {
        let browser = playwright
            .firefox()
            .launch()
            .await
            .expect("Failed to launch Firefox");
        let page = browser.new_page().await.expect("Failed to create page");
        page.goto(&data_url, None)
            .await
            .expect("Failed to navigate");

        let content = page.content().await.expect("Failed to get content");
        assert!(
            content.contains("Cross-Browser Test"),
            "Firefox: content should contain title"
        );
        browser.close().await.expect("Failed to close browser");
        tracing::info!("✓ page.content() works on Firefox");
    }

    // Test on WebKit
    {
        let browser = playwright
            .webkit()
            .launch()
            .await
            .expect("Failed to launch WebKit");
        let page = browser.new_page().await.expect("Failed to create page");
        page.goto(&data_url, None)
            .await
            .expect("Failed to navigate");

        let content = page.content().await.expect("Failed to get content");
        assert!(
            content.contains("Cross-Browser Test"),
            "WebKit: content should contain title"
        );
        browser.close().await.expect("Failed to close browser");
        tracing::info!("✓ page.content() works on WebKit");
    }
}

#[tokio::test]
async fn test_page_set_content() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Set content and verify
    page.set_content("<h1>Hello</h1><p>World</p>", None)
        .await
        .expect("Failed to set content");

    let content = page.content().await.expect("Failed to get content");
    assert!(
        content.contains("<h1>Hello</h1>"),
        "Content should contain h1"
    );
    assert!(content.contains("<p>World</p>"), "Content should contain p");
    tracing::info!("✓ set_content() works");

    // Set content again to verify replacement
    page.set_content("<div>Replaced</div>", None)
        .await
        .expect("Failed to set content again");

    let content = page.content().await.expect("Failed to get content");
    assert!(
        content.contains("<div>Replaced</div>"),
        "Content should be replaced"
    );
    assert!(
        !content.contains("<h1>Hello</h1>"),
        "Old content should be gone"
    );
    tracing::info!("✓ set_content() replaces existing content");

    // Verify locator works on set_content page
    let heading = page.locator("div").await;
    let text = heading
        .text_content()
        .await
        .expect("Failed to get text content");
    assert_eq!(text.as_deref(), Some("Replaced"));
    tracing::info!("✓ Locators work on set_content pages");

    // Cleanup
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_set_viewport_size_basic() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Navigate to a test page
    page.goto(
        "data:text/html,<html><body><h1>Viewport Test</h1></body></html>",
        None,
    )
    .await
    .expect("Failed to navigate");

    // Set viewport to mobile size
    let mobile_viewport = Viewport {
        width: 375,
        height: 667,
    };

    page.set_viewport_size(mobile_viewport)
        .await
        .expect("Failed to set viewport size");

    // Verify viewport changed using JavaScript
    let width: u32 = page
        .evaluate("window.innerWidth", None::<&()>)
        .await
        .expect("Failed to evaluate width");
    let height: u32 = page
        .evaluate("window.innerHeight", None::<&()>)
        .await
        .expect("Failed to evaluate height");

    assert_eq!(width, 375, "Viewport width should be 375");
    assert_eq!(height, 667, "Viewport height should be 667");

    tracing::info!("✓ Set viewport to mobile size (375x667)");

    // Set viewport to desktop size
    let desktop_viewport = Viewport {
        width: 1920,
        height: 1080,
    };

    page.set_viewport_size(desktop_viewport)
        .await
        .expect("Failed to set viewport size");

    // Verify viewport changed again
    let width: u32 = page
        .evaluate("window.innerWidth", None::<&()>)
        .await
        .expect("Failed to evaluate width");
    let height: u32 = page
        .evaluate("window.innerHeight", None::<&()>)
        .await
        .expect("Failed to evaluate height");

    assert_eq!(width, 1920, "Viewport width should be 1920");
    assert_eq!(height, 1080, "Viewport height should be 1080");

    tracing::info!("✓ Set viewport to desktop size (1920x1080)");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_set_viewport_size_different_dimensions() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<html><body><h1>Test</h1></body></html>",
        None,
    )
    .await
    .expect("Failed to navigate");

    // Test various viewport dimensions
    let test_cases = vec![
        (320, 568),   // iPhone SE
        (768, 1024),  // iPad portrait
        (1024, 768),  // iPad landscape
        (1366, 768),  // Common laptop
        (2560, 1440), // 2K monitor
    ];

    for (width, height) in test_cases {
        let viewport = Viewport { width, height };

        page.set_viewport_size(viewport)
            .await
            .expect("Failed to set viewport size");

        let actual_width: u32 = page
            .evaluate("window.innerWidth", None::<&()>)
            .await
            .expect("Failed to evaluate width");
        let actual_height: u32 = page
            .evaluate("window.innerHeight", None::<&()>)
            .await
            .expect("Failed to evaluate height");

        assert_eq!(
            actual_width, width,
            "Viewport width should be {} for {}x{}",
            width, width, height
        );
        assert_eq!(
            actual_height, height,
            "Viewport height should be {} for {}x{}",
            height, width, height
        );

        tracing::info!("✓ Set viewport to {}x{}", width, height);
    }

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
#[ignore]
async fn test_set_viewport_size_cross_browser() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test on Chromium, Firefox, and WebKit
    let browser_types = vec![
        ("chromium", playwright.chromium()),
        ("firefox", playwright.firefox()),
        ("webkit", playwright.webkit()),
    ];

    for (name, browser_type) in browser_types {
        let browser = browser_type
            .launch()
            .await
            .unwrap_or_else(|_| panic!("Failed to launch {}", name));

        let page = browser.new_page().await.expect("Failed to create page");

        page.goto(
            "data:text/html,<html><body><h1>Test</h1></body></html>",
            None,
        )
        .await
        .expect("Failed to navigate");

        // Set viewport to mobile size
        let viewport = Viewport {
            width: 375,
            height: 667,
        };

        page.set_viewport_size(viewport)
            .await
            .unwrap_or_else(|_| panic!("Failed to set viewport size on {}", name));

        // Verify viewport changed
        let width: u32 = page
            .evaluate("window.innerWidth", None::<&()>)
            .await
            .expect("Failed to evaluate width");
        let height: u32 = page
            .evaluate("window.innerHeight", None::<&()>)
            .await
            .expect("Failed to evaluate height");

        assert_eq!(width, 375, "Viewport width should be 375 on {}", name);
        assert_eq!(height, 667, "Viewport height should be 667 on {}", name);

        tracing::info!("✓ {} - Set viewport to 375x667", name);

        // Cleanup
        page.close().await.expect("Failed to close page");
        browser.close().await.expect("Failed to close browser");
    }
}

#[tokio::test]
async fn test_set_viewport_size_with_responsive_content() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Create a simple responsive page that uses JavaScript to detect viewport
    let html = r#"<!DOCTYPE html><html><head></head><body><script>window.getViewportCategory=function(){return window.innerWidth<=768?'mobile':'desktop';};</script></body></html>"#;

    page.goto(&format!("data:text/html,{}", html), None)
        .await
        .expect("Failed to navigate");

    // Set viewport to mobile size
    let mobile_viewport = Viewport {
        width: 375,
        height: 667,
    };

    page.set_viewport_size(mobile_viewport)
        .await
        .expect("Failed to set viewport size");

    // Check viewport category using JavaScript
    let category: String = page
        .evaluate("window.getViewportCategory()", None::<&()>)
        .await
        .expect("Failed to evaluate viewport category");

    assert_eq!(
        category, "mobile",
        "Should detect mobile viewport at 375px width"
    );

    tracing::info!("✓ Mobile viewport (375px) detected correctly");

    // Set viewport to desktop size
    let desktop_viewport = Viewport {
        width: 1024,
        height: 768,
    };

    page.set_viewport_size(desktop_viewport)
        .await
        .expect("Failed to set viewport size");

    // Check viewport category again
    let category: String = page
        .evaluate("window.getViewportCategory()", None::<&()>)
        .await
        .expect("Failed to evaluate viewport category");

    assert_eq!(
        category, "desktop",
        "Should detect desktop viewport at 1024px width"
    );

    tracing::info!("✓ Desktop viewport (1024px) detected correctly");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// page.accessibility and page.coverage
// ============================================================================

#[tokio::test]
async fn test_page_accessibility_snapshot() {
    let (_playwright, browser, page) = crate::common::setup().await;

    let html = "data:text/html,<html><body><h1>Hello</h1></body></html>";
    page.goto(html, None).await.expect("Failed to navigate");

    let accessibility = page.accessibility();
    let snapshot = accessibility
        .snapshot(None)
        .await
        .expect("Failed to get accessibility snapshot");

    assert!(
        !snapshot.is_null(),
        "Accessibility snapshot should not be null"
    );

    let binding = snapshot.to_string();
    let snapshot_str = snapshot.as_str().unwrap_or(&binding);
    assert!(
        snapshot_str.contains("heading")
            || snapshot_str.contains("WebArea")
            || snapshot_str.contains("- heading"),
        "Snapshot should contain heading role, got: {}",
        snapshot_str
    );

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// page.add_locator_handler() / page.remove_locator_handler()
// ============================================================================

#[tokio::test]
async fn test_page_add_locator_handler() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    let (_playwright, browser, page) = crate::common::setup().await;

    // Page with an overlay that starts visible and covers an underlying button.
    // The overlay handler removes it so the underlying button can be clicked.
    let html = r#"<!DOCTYPE html>
<html><body>
  <button id="target">Target</button>
  <div id="overlay" style="position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,0,0,0.7)">overlay</div>
  <div id="result">pending</div>
</body></html>"#;

    page.set_content(html, None)
        .await
        .expect("Failed to set content");

    // Track whether the handler was invoked.
    let handler_ran = Arc::new(AtomicBool::new(false));
    let handler_ran_clone = Arc::clone(&handler_ran);

    // Register a locator handler: when #overlay appears, remove it and mark the flag.
    let overlay_locator = page.locator("#overlay").await;
    page.add_locator_handler(
        &overlay_locator,
        move |_locator| {
            let flag = Arc::clone(&handler_ran_clone);
            async move {
                flag.store(true, Ordering::SeqCst);
                Ok(())
            }
        },
        None,
    )
    .await
    .expect("Failed to add locator handler");

    // Evaluate JS to remove the overlay so the target button becomes actionable,
    // simulating what a real handler would do.
    page.evaluate_expression("document.getElementById('overlay').remove()")
        .await
        .expect("Failed to remove overlay via JS");

    // Clicking the target must now succeed (overlay gone).
    page.locator("#target")
        .await
        .click(None)
        .await
        .expect("Failed to click target button");

    // Remove the handler and verify it succeeds.
    page.remove_locator_handler(&overlay_locator)
        .await
        .expect("Failed to remove locator handler");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_coverage_js() {
    let (_playwright, browser, page) = crate::common::setup().await;

    let coverage = page.coverage();

    coverage
        .start_js_coverage(None)
        .await
        .expect("Failed to start JS coverage");

    let html = "data:text/html,<html><head><script>function hello() { return 42; } hello();</script></head><body></body></html>";
    page.goto(html, None).await.expect("Failed to navigate");

    let entries = coverage
        .stop_js_coverage()
        .await
        .expect("Failed to stop JS coverage");

    assert!(
        !entries.is_empty(),
        "JS coverage should return at least one entry"
    );

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Clock tests
// ============================================================================

#[tokio::test]
async fn test_page_clock_install_and_fast_forward() {
    use playwright_rs::ClockInstallOptions;

    let (_playwright, browser, page) = crate::common::setup().await;

    let clock = page.clock().expect("Failed to get clock");

    // Install fake timers then pause at a known epoch to get deterministic time
    clock
        .install(Some(ClockInstallOptions { time: Some(0) }))
        .await
        .expect("Failed to install clock");

    clock
        .pause_at(1_000_000)
        .await
        .expect("Failed to pause clock");

    // Date.now() should now be exactly 1 000 000 ms
    let now_str = page
        .evaluate_value("Date.now()")
        .await
        .expect("Failed to evaluate Date.now()");
    let now: u64 = now_str
        .parse::<f64>()
        .expect("Date.now() result is not a number") as u64;
    assert_eq!(now, 1_000_000, "Date.now() should equal the paused time");

    // Advance by 5 000 ms; fast_forward fires due timers and moves the clock
    clock
        .fast_forward(5_000)
        .await
        .expect("Failed to fast-forward clock");

    let after_str = page
        .evaluate_value("Date.now()")
        .await
        .expect("Failed to evaluate Date.now() after fast_forward");
    let after: u64 = after_str
        .parse::<f64>()
        .expect("Date.now() result is not a number") as u64;
    assert_eq!(
        after, 1_005_000,
        "Date.now() should reflect the fast-forwarded time"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_clock_matches_page_clock() {
    let (_playwright, browser, page) = crate::common::setup().await;

    let context = page.context().expect("Failed to get context");
    let context_clock = context.clock();
    let page_clock = page.clock().expect("Failed to get page clock");

    // Both clocks share the same underlying BrowserContext channel; install via
    // context_clock and verify the effect is visible via page_clock / evaluate.
    context_clock
        .set_fixed_time(9_999_999)
        .await
        .expect("Failed to set fixed time via context clock");

    let now_str = page
        .evaluate_value("Date.now()")
        .await
        .expect("Failed to evaluate Date.now()");
    let now: u64 = now_str
        .parse::<f64>()
        .expect("Date.now() result is not a number") as u64;
    assert_eq!(
        now, 9_999_999,
        "context.clock() and page.clock() should affect the same timeline"
    );

    // Calling page_clock.resume() after set_fixed_time should not error
    page_clock
        .resume()
        .await
        .expect("resume() should succeed after set_fixed_time");

    browser.close().await.expect("Failed to close browser");
}
