use crate::test_server::TestServer;
use playwright_rs::protocol::{
    AddScriptTagOptions, ColorScheme, EmulateMediaOptions, Media, Playwright,
};

// ============================================================================
// page.set_extra_http_headers()
// ============================================================================

/// Exercises set_extra_http_headers with a single header and with multiple headers.
#[tokio::test]
async fn test_page_set_extra_http_headers() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // Single custom header is echoed by the server
    let mut headers = std::collections::HashMap::new();
    headers.insert(
        "x-page-custom-header".to_string(),
        "page-header-value-42".to_string(),
    );
    page.set_extra_http_headers(headers)
        .await
        .expect("Failed to set extra HTTP headers on page");

    page.goto(&format!("{}/echo-headers", server.url()), None)
        .await
        .expect("Failed to navigate");

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

    // Multiple headers are all echoed by the server
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

/// Test that emulate_media(Print) causes matchMedia('print') to match.
///
/// Kept separate from the dark color-scheme test because media state persists
/// within a page session — mixing both assertions in one page would confuse results.
#[tokio::test]
async fn test_page_emulate_media_print() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    page.emulate_media(Some(
        EmulateMediaOptions::builder().media(Media::Print).build(),
    ))
    .await
    .expect("Failed to emulate media");

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

/// Test that emulate_media(ColorScheme::Dark) causes matchMedia to report dark mode.
///
/// Kept separate from the print-media test because media state persists within a
/// page session.
#[tokio::test]
async fn test_page_emulate_media_color_scheme_dark() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    page.emulate_media(Some(
        EmulateMediaOptions::builder()
            .color_scheme(ColorScheme::Dark)
            .build(),
    ))
    .await
    .expect("Failed to emulate dark color scheme");

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

/// Test that emulate_media(None) succeeds without error.
#[tokio::test]
async fn test_page_emulate_media_none() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.emulate_media(None)
        .await
        .expect("emulate_media(None) should not fail");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// page.pdf()
// ============================================================================

/// Exercises pdf() with no options and with explicit PdfOptions in one session.
#[tokio::test]
async fn test_page_pdf() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // pdf() with no options returns valid PDF bytes
    let pdf_bytes = page.pdf(None).await.expect("Failed to generate PDF");

    assert!(!pdf_bytes.is_empty(), "PDF bytes should not be empty");
    assert_eq!(
        &pdf_bytes[0..4],
        b"%PDF",
        "Generated bytes should start with PDF magic bytes"
    );

    // pdf() with explicit options (A4, landscape, print_background) also returns valid PDF
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

/// Exercises add_script_tag with valid content and error cases in one session.
#[tokio::test]
async fn test_page_add_script_tag() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    // Script tag with inline content executes and sets a window variable
    page.add_script_tag(Some(
        AddScriptTagOptions::builder()
            .content("window.scriptTagExecuted = 'yes_it_ran';")
            .build(),
    ))
    .await
    .expect("Failed to add script tag");

    let value = page
        .evaluate_value("window.scriptTagExecuted")
        .await
        .expect("Failed to evaluate window variable");

    assert!(
        value.contains("yes_it_ran"),
        "Script tag should have executed and set window variable. Got: {}",
        value
    );

    // add_script_tag with empty options fails with a descriptive error
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

    // add_script_tag(None) also fails
    let result = page.add_script_tag(None).await;
    assert!(result.is_err(), "add_script_tag(None) should fail");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
#[ignore]
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

    page.add_script_tag(Some(
        AddScriptTagOptions::builder()
            .content("window.firefoxScriptRan = true;")
            .build(),
    ))
    .await
    .expect("Failed to add script tag in Firefox");

    let value = page
        .evaluate_value("window.firefoxScriptRan")
        .await
        .expect("Failed to evaluate");

    assert_eq!(value.trim(), "true", "Script should have run in Firefox");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
#[ignore]
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

    page.add_script_tag(Some(
        AddScriptTagOptions::builder()
            .content("window.webkitScriptRan = true;")
            .build(),
    ))
    .await
    .expect("Failed to add script tag in WebKit");

    let value = page
        .evaluate_value("window.webkitScriptRan")
        .await
        .expect("Failed to evaluate");

    assert_eq!(value.trim(), "true", "Script should have run in WebKit");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_pick_locator_cancel_releases_handle() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content("<html><body><h1>cancel test</h1></body></html>", None)
        .await
        .expect("Failed to set content");

    let page_for_pick = page.clone();
    let pick_handle = tokio::spawn(async move { page_for_pick.pick_locator().await });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    page.cancel_pick_locator()
        .await
        .expect("cancel_pick_locator should succeed");

    // Whether pick_locator returns Ok or Err on cancel is server-defined;
    // the contract under test is that cancel unblocks the call.
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(5), pick_handle)
        .await
        .expect("pick_locator did not resolve within 5s of cancel");
    let _ = outcome.expect("spawned task panicked");

    browser.close().await.expect("Failed to close browser");
}
