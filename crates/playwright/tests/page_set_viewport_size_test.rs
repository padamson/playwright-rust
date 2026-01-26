// Tests for page.set_viewport_size() method
//
// Tests viewport resizing for responsive testing.

use playwright_rs::protocol::{Playwright, Viewport};

mod common;

#[tokio::test]
async fn test_set_viewport_size_basic() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

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
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

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
async fn test_set_viewport_size_cross_browser() {
    common::init_tracing();
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
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

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
