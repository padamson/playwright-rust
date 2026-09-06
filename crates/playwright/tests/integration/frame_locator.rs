use crate::test_server::TestServer;

// ============================================================================
// Core: Page::frame_locator() + locator()
// ============================================================================

/// Basic FrameLocator: click a button inside an iframe
#[tokio::test]
async fn test_frame_locator_click_button() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    // Click button inside the "content" iframe
    let frame = page.frame_locator("iframe[name='content']");
    frame.locator("#frame-btn").click(None).await?;

    // Verify button text changed
    let text = frame.locator("#frame-btn").text_content().await?;
    assert_eq!(text, Some("clicked".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// FrameLocator reads text from inside iframe
#[tokio::test]
async fn test_frame_locator_text_content() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']");
    let heading = frame.locator("h1").text_content().await?;
    assert_eq!(heading, Some("Inside Frame".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// get_by_* methods
// ============================================================================

/// get_by_text inside iframe
#[tokio::test]
async fn test_frame_locator_get_by_text() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']");
    let btn = frame.get_by_text("Click Me", false);
    let text = btn.text_content().await?;
    assert_eq!(text, Some("Click Me".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// get_by_label inside iframe
#[tokio::test]
async fn test_frame_locator_get_by_label() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']");
    let input = frame.get_by_label("Email", false);
    input.fill("test@example.com", None).await?;
    let value = input.input_value(None).await?;
    assert_eq!(value, "test@example.com");

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// get_by_test_id inside iframe
#[tokio::test]
async fn test_frame_locator_get_by_test_id() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']");
    let btn = frame.get_by_test_id("frame-submit");
    let text = btn.text_content().await?;
    assert_eq!(text, Some("Submit".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// Nested FrameLocator
// ============================================================================

/// Nested frame_locator: iframe within iframe
#[tokio::test]
async fn test_frame_locator_nested() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/nested-iframe.html", server.url()), None)
        .await?;

    // Navigate: outer page → #outer iframe → #innermost iframe → h1
    let inner = page.frame_locator("#outer").frame_locator("#innermost");
    let heading = inner.locator("h1").text_content().await?;
    assert_eq!(heading, Some("Inside Frame".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// owner property
// ============================================================================

/// owner() returns a Locator for the iframe element itself
#[tokio::test]
async fn test_frame_locator_owner() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let frame = page.frame_locator("iframe[name='content']");
    let iframe_element = frame.owner();
    let name = iframe_element.get_attribute("name").await?;
    assert_eq!(name, Some("content".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// Locator::frame_locator()
// ============================================================================

/// frame_locator() from a Locator (scoped)
#[tokio::test]
async fn test_locator_frame_locator() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    // Use locator("body") to scope, then frame_locator into iframe
    let heading = page
        .locator("body")
        .frame_locator("iframe[name='content']")
        .locator("h1")
        .text_content()
        .await?;
    assert_eq!(heading, Some("Inside Frame".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Without a selector, a frame locator searches every frame in the subtree,
/// so the iframe does not have to be located first.
#[tokio::test]
async fn frame_locator_without_a_selector_searches_every_frame()
-> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let button = page.frame_locator(None).locator("#frame-btn");
    button.click(None).await?;
    assert_eq!(button.text_content().await?, Some("clicked".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// The same, from a frame rather than the page.
#[tokio::test]
async fn frame_frame_locator_without_a_selector() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    let main = page.main_frame().await?;
    let button = main
        .frame_locator(None)
        .locator("#btn2")
        .text_content()
        .await?;
    assert_eq!(button, Some("Other Button".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// The rest of the locator still resolves inside one frame, so a selector
/// that matches in several is an error rather than an arbitrary pick.
#[tokio::test]
async fn frame_locator_without_a_selector_refuses_a_match_in_several_frames()
-> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    // Both iframes have an `h1`.
    let error = page
        .frame_locator(None)
        .locator("h1")
        .text_content()
        .await
        .expect_err("a match in several frames must be refused");
    assert!(
        error.to_string().contains("multiple frames"),
        "unexpected error: {error}"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// `frame_locator(None)` names no iframe, so there is no nth frame to pick.
#[tokio::test]
#[should_panic(expected = "Selecting the nth frame is not allowed")]
async fn nth_on_an_any_frame_locator_is_refused() {
    let (_pw, browser, page) = crate::common::setup().await;
    let _ = page.frame_locator(None).first();
    browser.close().await.expect("close browser");
}
