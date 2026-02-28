// Integration tests for add_init_script functionality
//
// Tests cover:
// - BrowserContext.add_init_script() - scripts applied to all pages in context
// - Page.add_init_script() - scripts applied to specific page
// - Multiple pages inheriting context scripts
// - Script execution before page scripts
// - Cross-browser compatibility

use crate::test_server::TestServer;
use playwright_rs::protocol::{AddStyleTagOptions, Playwright};

#[tokio::test]
async fn test_add_init_script_on_context() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Add init script to context - will apply to all pages
    context
        .add_init_script(
            r#"
            window.playwrightInitialized = true;
            window.customTimestamp = Date.now();
            console.log('Init script from context executed!');
            "#,
        )
        .await
        .expect("Failed to add init script");

    let page = context.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    // Wait for page to be fully loaded and script to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let initialized = page
        .evaluate_value("window.playwrightInitialized")
        .await
        .expect("Failed to evaluate script");

    let timestamp = page
        .evaluate_value("window.customTimestamp")
        .await
        .expect("Failed to evaluate timestamp");

    assert_eq!(
        initialized.trim(),
        "true",
        "Property playwrightInitialized should be true"
    );
    assert!(
        !timestamp.is_empty() && timestamp.parse::<f64>().is_ok(),
        "Timestamp should be a number, but got: {}",
        timestamp
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_init_script_multiple_pages() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Add init script at context level
    context
        .add_init_script(
            r#"
            window.sharedCounter = 42;
            window.contextId = "test-context";
            "#,
        )
        .await
        .expect("Failed to add init script");

    // Create first page
    let page1 = context.new_page().await.expect("Failed to create page 1");
    page1
        .goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate page 1")
        .expect("Expected a response");

    // Wait for script to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Create second page
    let page2 = context.new_page().await.expect("Failed to create page 2");
    page2
        .goto(&format!("{}/form.html", server.url()), None)
        .await
        .expect("Failed to navigate page 2")
        .expect("Expected a response");

    // Wait for script to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Both pages should have the same init script values
    let counter1 = page1
        .evaluate_value("window.sharedCounter")
        .await
        .expect("Failed to evaluate counter on page 1");

    let counter2 = page2
        .evaluate_value("window.sharedCounter")
        .await
        .expect("Failed to evaluate counter on page 2");

    let context_id1 = page1
        .evaluate_value("window.contextId")
        .await
        .expect("Failed to evaluate contextId on page 1");

    let context_id2 = page2
        .evaluate_value("window.contextId")
        .await
        .expect("Failed to evaluate contextId on page 2");

    assert_eq!(counter1.trim(), "42", "Counter on page 1 should be 42");
    assert_eq!(counter2.trim(), "42", "Counter on page 2 should be 42");
    assert_eq!(
        context_id1.trim().trim_matches('"'),
        "test-context",
        "Context ID on page 1 is incorrect"
    );
    assert_eq!(
        context_id2.trim().trim_matches('"'),
        "test-context",
        "Context ID on page 2 is incorrect"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_init_script_on_page() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Add init script directly to page
    page.add_init_script(
        r#"
        window.pageInitialized = true;
        window.pageCounter = 999;
        console.log('Init script from page executed!');
        "#,
    )
    .await
    .expect("Failed to add init script on page");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    // Wait for page to be fully loaded and script to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let initialized = page
        .evaluate_value("window.pageInitialized")
        .await
        .expect("Failed to evaluate pageInitialized");

    let counter = page
        .evaluate_value("window.pageCounter")
        .await
        .expect("Failed to evaluate pageCounter");

    assert_eq!(
        initialized.trim(),
        "true",
        "Property pageInitialized should be true"
    );
    assert_eq!(counter.trim(), "999", "Counter should be 999");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_init_script_chromium() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    page.add_init_script("window.browserType = 'chromium';")
        .await
        .expect("Failed to add init script");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let browser_type = page
        .evaluate_value("window.browserType")
        .await
        .expect("Failed to evaluate browserType");

    assert_eq!(
        browser_type.trim().trim_matches('"'),
        "chromium",
        "Browser type should be chromium"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_init_script_firefox() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    page.add_init_script("window.browserType = 'firefox';")
        .await
        .expect("Failed to add init script");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let browser_type = page
        .evaluate_value("window.browserType")
        .await
        .expect("Failed to evaluate browserType");

    assert_eq!(
        browser_type.trim().trim_matches('"'),
        "firefox",
        "Browser type should be firefox"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_init_script_webkit() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    page.add_init_script("window.browserType = 'webkit';")
        .await
        .expect("Failed to add init script");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let browser_type = page
        .evaluate_value("window.browserType")
        .await
        .expect("Failed to evaluate browserType");

    assert_eq!(
        browser_type.trim().trim_matches('"'),
        "webkit",
        "Browser type should be webkit"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Merged from: add_style_tag_test.rs
// ============================================================================

// Integration tests for add_style_tag functionality
//
// Tests cover:
// - Page.add_style_tag() with inline content - CSS injection and verification
// - Multiple style tags - sequential CSS injection
// - Style tag with URL parameter
// - Error cases - invalid options

#[tokio::test]
async fn test_add_style_tag_with_content() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    // Wait for page to be fully loaded
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Inject CSS to change background color
    page.add_style_tag(
        AddStyleTagOptions::builder()
            .content(
                r#"
        body {
            background-color: rgb(255, 0, 0) !important;
        }
        "#,
            )
            .build(),
    )
    .await
    .expect("Failed to add style tag");

    // Give browser time to apply styles
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let bg_color = page
        .evaluate_value("window.getComputedStyle(document.body).backgroundColor")
        .await
        .expect("Failed to evaluate background color");

    assert!(
        bg_color.contains("rgb(255, 0, 0)") || bg_color.contains("rgb(255,0,0)"),
        "Background color should be red, got: {}",
        bg_color
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_style_tag_multiple_styles() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    // Wait for page to be fully loaded
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Add first style tag
    page.add_style_tag(
        AddStyleTagOptions::builder()
            .content(
                r#"
        body {
            font-size: 32px !important;
        }
        "#,
            )
            .build(),
    )
    .await
    .expect("Failed to add first style tag");

    // Add second style tag
    page.add_style_tag(
        AddStyleTagOptions::builder()
            .content(
                r#"
        body {
            color: rgb(0, 255, 0) !important;
        }
        "#,
            )
            .build(),
    )
    .await
    .expect("Failed to add second style tag");

    // Give browser time to apply styles
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let font_size = page
        .evaluate_value("window.getComputedStyle(document.body).fontSize")
        .await
        .expect("Failed to evaluate font size");

    let color = page
        .evaluate_value("window.getComputedStyle(document.body).color")
        .await
        .expect("Failed to evaluate color");

    assert!(
        font_size.contains("32px"),
        "Font size should be 32px, got: {}",
        font_size
    );
    assert!(
        color.contains("rgb(0, 255, 0)") || color.contains("rgb(0,255,0)"),
        "Color should be green, got: {}",
        color
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_style_tag_after_navigation() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    // Wait for page to be fully loaded
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Add style tag after navigation
    page.add_style_tag(
        AddStyleTagOptions::builder()
            .content(
                r#"
        body {
            margin: 0px !important;
            padding: 0px !important;
        }
        "#,
            )
            .build(),
    )
    .await
    .expect("Failed to add style tag");

    // Give browser time to apply styles
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let margin = page
        .evaluate_value("window.getComputedStyle(document.body).margin")
        .await
        .expect("Failed to evaluate margin");

    let padding = page
        .evaluate_value("window.getComputedStyle(document.body).padding")
        .await
        .expect("Failed to evaluate padding");

    assert!(
        margin.contains("0px"),
        "Margin should be 0px, got: {}",
        margin
    );
    assert!(
        padding.contains("0px"),
        "Padding should be 0px, got: {}",
        padding
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_style_tag_error_no_options() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Try to add style tag with no content, url, or path
    let result = page
        .add_style_tag(AddStyleTagOptions::builder().build())
        .await;

    assert!(result.is_err(), "Should fail when no options are provided");

    if let Err(e) = result {
        let error_msg = format!("{}", e);
        assert!(
            error_msg.contains("At least one"),
            "Error should mention that at least one option is required, got: {}",
            error_msg
        );
    }

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_add_style_tag_chromium() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    page.add_style_tag(
        AddStyleTagOptions::builder()
            .content("body { background-color: blue !important; }")
            .build(),
    )
    .await
    .expect("Failed to add style tag");

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let bg_color = page
        .evaluate_value("window.getComputedStyle(document.body).backgroundColor")
        .await
        .expect("Failed to evaluate background color");

    assert!(
        bg_color.contains("blue") || bg_color.contains("0, 0, 255"),
        "Background should be blue in Chromium"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_style_tag_firefox() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    page.add_style_tag(
        AddStyleTagOptions::builder()
            .content("body { background-color: green !important; }")
            .build(),
    )
    .await
    .expect("Failed to add style tag");

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let bg_color = page
        .evaluate_value("window.getComputedStyle(document.body).backgroundColor")
        .await
        .expect("Failed to evaluate background color");

    assert!(
        bg_color.contains("green")
            || bg_color.contains("0, 128, 0")
            || bg_color.contains("0, 255, 0"),
        "Background should be green in Firefox"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_style_tag_webkit() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    page.add_style_tag(
        AddStyleTagOptions::builder()
            .content("body { background-color: yellow !important; }")
            .build(),
    )
    .await
    .expect("Failed to add style tag");

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let bg_color = page
        .evaluate_value("window.getComputedStyle(document.body).backgroundColor")
        .await
        .expect("Failed to evaluate background color");

    assert!(
        bg_color.contains("yellow") || bg_color.contains("255, 255, 0"),
        "Background should be yellow in WebKit"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
