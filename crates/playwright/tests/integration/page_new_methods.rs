// Integration tests for new Page methods:
//   - page.set_extra_http_headers()
//   - page.emulate_media()
//   - page.pdf()
//   - page.add_script_tag()
//
// TDD: Tests written FIRST before any implementation.

use crate::test_server::TestServer;
use playwright_rs::protocol::{
    AddScriptTagOptions, ColorScheme, EmulateMediaOptions, Media, Playwright,
};

// ============================================================================
// page.set_extra_http_headers()
// ============================================================================

#[tokio::test]
async fn test_page_set_extra_http_headers() {
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

    // Set a custom header on the page
    let mut headers = std::collections::HashMap::new();
    headers.insert(
        "x-page-custom-header".to_string(),
        "page-header-value-42".to_string(),
    );
    page.set_extra_http_headers(headers)
        .await
        .expect("Failed to set extra HTTP headers on page");

    // Navigate to the echo-headers endpoint
    page.goto(&format!("{}/echo-headers", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Read the echoed headers
    let headers_json = page
        .evaluate_value("document.getElementById('headers').textContent")
        .await
        .expect("Failed to evaluate headers");

    assert!(
        headers_json.contains("x-page-custom-header"),
        "Custom header name should be present. Got: {}",
        headers_json
    );
    assert!(
        headers_json.contains("page-header-value-42"),
        "Custom header value should be present. Got: {}",
        headers_json
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_set_extra_http_headers_multiple() {
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

    let mut headers = std::collections::HashMap::new();
    headers.insert("x-page-alpha".to_string(), "alpha-val".to_string());
    headers.insert("x-page-beta".to_string(), "beta-val".to_string());
    page.set_extra_http_headers(headers)
        .await
        .expect("Failed to set extra HTTP headers");

    page.goto(&format!("{}/echo-headers", server.url()), None)
        .await
        .expect("Failed to navigate");

    let headers_json = page
        .evaluate_value("document.getElementById('headers').textContent")
        .await
        .expect("Failed to evaluate headers");

    assert!(
        headers_json.contains("x-page-alpha"),
        "First header should be present. Got: {}",
        headers_json
    );
    assert!(
        headers_json.contains("x-page-beta"),
        "Second header should be present. Got: {}",
        headers_json
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// page.emulate_media()
// ============================================================================

#[tokio::test]
async fn test_page_emulate_media_print() {
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

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Set media to print
    page.emulate_media(Some(
        EmulateMediaOptions::builder().media(Media::Print).build(),
    ))
    .await
    .expect("Failed to emulate media");

    // Verify via matchMedia
    let matches = page
        .evaluate_value("window.matchMedia('print').matches")
        .await
        .expect("Failed to evaluate matchMedia");

    assert_eq!(
        matches.trim(),
        "true",
        "print media should match after emulating print. Got: {}",
        matches
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_emulate_media_color_scheme_dark() {
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

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Set color scheme to dark
    page.emulate_media(Some(
        EmulateMediaOptions::builder()
            .color_scheme(ColorScheme::Dark)
            .build(),
    ))
    .await
    .expect("Failed to emulate dark color scheme");

    // Verify via matchMedia
    let matches = page
        .evaluate_value("window.matchMedia('(prefers-color-scheme: dark)').matches")
        .await
        .expect("Failed to evaluate matchMedia");

    assert_eq!(
        matches.trim(),
        "true",
        "dark color scheme should match. Got: {}",
        matches
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_emulate_media_none() {
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

    // Calling emulate_media with None options should work without error
    page.emulate_media(None)
        .await
        .expect("emulate_media(None) should not fail");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// page.pdf()
// ============================================================================

#[tokio::test]
async fn test_page_pdf_basic() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    // PDF only works in Chromium
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let pdf_bytes = page.pdf(None).await.expect("Failed to generate PDF");

    assert!(!pdf_bytes.is_empty(), "PDF bytes should not be empty");
    // PDF files start with "%PDF"
    assert_eq!(
        &pdf_bytes[0..4],
        b"%PDF",
        "Generated bytes should start with PDF magic bytes"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_pdf_with_options() {
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

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    use playwright_rs::protocol::PdfOptions;

    let pdf_bytes = page
        .pdf(Some(
            PdfOptions::builder()
                .format("A4".to_string())
                .landscape(true)
                .print_background(true)
                .build(),
        ))
        .await
        .expect("Failed to generate PDF with options");

    assert!(
        !pdf_bytes.is_empty(),
        "PDF with options should return bytes"
    );
    assert_eq!(&pdf_bytes[0..4], b"%PDF", "Should be a valid PDF");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// page.add_script_tag()
// ============================================================================

#[tokio::test]
async fn test_page_add_script_tag_with_content() {
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

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Add a script tag that sets a window variable
    page.add_script_tag(Some(
        AddScriptTagOptions::builder()
            .content("window.scriptTagExecuted = 'yes_it_ran';")
            .build(),
    ))
    .await
    .expect("Failed to add script tag");

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let value = page
        .evaluate_value("window.scriptTagExecuted")
        .await
        .expect("Failed to evaluate window variable");

    assert!(
        value.contains("yes_it_ran"),
        "Script tag should have executed and set window variable. Got: {}",
        value
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_add_script_tag_error_no_options() {
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

    // Should fail with no options (no url, path, or content)
    let result = page
        .add_script_tag(Some(AddScriptTagOptions::builder().build()))
        .await;
    assert!(result.is_err(), "Should fail when no options provided");

    if let Err(e) = result {
        let msg = format!("{}", e);
        assert!(
            msg.contains("At least one"),
            "Error should mention that at least one option is required. Got: {}",
            msg
        );
    }

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_add_script_tag_none_options() {
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

    // Passing None should also fail (no content to inject)
    let result = page.add_script_tag(None).await;
    assert!(result.is_err(), "add_script_tag(None) should fail");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_add_script_tag_cross_browser_chromium() {
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

    page.add_script_tag(Some(
        AddScriptTagOptions::builder()
            .content("window.chromiumScriptRan = true;")
            .build(),
    ))
    .await
    .expect("Failed to add script tag in Chromium");

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let value = page
        .evaluate_value("window.chromiumScriptRan")
        .await
        .expect("Failed to evaluate");

    assert_eq!(value.trim(), "true", "Script should have run in Chromium");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_add_script_tag_cross_browser_firefox() {
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

    page.add_script_tag(Some(
        AddScriptTagOptions::builder()
            .content("window.firefoxScriptRan = true;")
            .build(),
    ))
    .await
    .expect("Failed to add script tag in Firefox");

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let value = page
        .evaluate_value("window.firefoxScriptRan")
        .await
        .expect("Failed to evaluate");

    assert_eq!(value.trim(), "true", "Script should have run in Firefox");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_add_script_tag_cross_browser_webkit() {
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

    page.add_script_tag(Some(
        AddScriptTagOptions::builder()
            .content("window.webkitScriptRan = true;")
            .build(),
    ))
    .await
    .expect("Failed to add script tag in WebKit");

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let value = page
        .evaluate_value("window.webkitScriptRan")
        .await
        .expect("Failed to evaluate");

    assert_eq!(value.trim(), "true", "Script should have run in WebKit");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
