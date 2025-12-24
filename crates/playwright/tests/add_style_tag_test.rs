// Integration tests for add_style_tag functionality
//
// Tests cover:
// - Page.add_style_tag() with inline content - CSS injection and verification
// - Multiple style tags - sequential CSS injection
// - Style tag with URL parameter
// - Error cases - invalid options

mod test_server;

use playwright_rs::protocol::{AddStyleTagOptions, Playwright};
use test_server::TestServer;

#[tokio::test]
async fn test_add_style_tag_with_content() {
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
