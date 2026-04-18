// Integration tests for Frame public API methods
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - frame.locator() - create a locator scoped to a frame
// - frame.get_by_text() / get_by_label() / get_by_placeholder() / get_by_alt_text()
// - frame.get_by_title() / get_by_test_id() / get_by_role()
// - frame.name() - name of the frame (empty string for main frame)
// - frame.page() - returns the owning Page
// - frame.is_detached() - false for attached frames
// - frame.parent_frame() - None for main frame
// - frame.evaluate_handle() - evaluates JS and returns a handle to the result
// - frame.child_frames() - returns child frame list

use crate::test_server::TestServer;
use playwright_rs::server::channel_owner::ChannelOwner;

// ============================================================================
// frame.locator()
// ============================================================================

/// Main frame locator: create a locator and use it to click a button
#[tokio::test]
async fn test_frame_locator() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/button.html", server.url()), None)
        .await?;

    let frame = page.main_frame().await?;

    // Frame::locator should create a Locator scoped to this frame.
    // Use the specific id selector to avoid strict-mode errors (button.html has 2 buttons).
    let btn = frame.locator("#btn");
    let text = btn.text_content().await?;
    assert_eq!(text, Some("Click me".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// frame.get_by_text()
// ============================================================================

/// Main frame get_by_text: find element by text content
#[tokio::test]
async fn test_frame_get_by_text() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/button.html", server.url()), None)
        .await?;

    let frame = page.main_frame().await?;

    // Frame::get_by_text should find elements containing the exact text.
    // button.html has two buttons: "Click me" and "Click me 2".
    // Use exact match for the first button's full text.
    let btn = frame.get_by_text("Click me", true);
    let text = btn.text_content().await?;
    assert_eq!(text, Some("Click me".to_string()));

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// frame.name()
// ============================================================================

/// Main frame name should be an empty string
#[tokio::test]
async fn test_frame_name() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let frame = page.main_frame().await?;

    // The main frame's name is an empty string in Playwright
    assert_eq!(frame.name(), "");

    browser.close().await?;
    Ok(())
}

// ============================================================================
// frame.page()
// ============================================================================

/// frame.page() returns the owning Page
#[tokio::test]
async fn test_frame_page() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let frame = page.main_frame().await?;

    // frame.page() should return the owning Page
    let frame_page = frame.page().expect("Frame should have an owning page");
    // Both page and frame_page refer to the same underlying page
    assert_eq!(frame_page.url(), page.url());

    browser.close().await?;
    Ok(())
}

// ============================================================================
// frame.is_detached()
// ============================================================================

/// Main frame is_detached() returns false
#[tokio::test]
async fn test_frame_is_detached() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let frame = page.main_frame().await?;

    // The main frame of an active page is not detached
    assert!(!frame.is_detached());

    browser.close().await?;
    Ok(())
}

// ============================================================================
// frame.parent_frame()
// ============================================================================

/// Main frame has no parent frame
#[tokio::test]
async fn test_frame_parent_frame() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let frame = page.main_frame().await?;

    // The main frame has no parent frame (it IS the top-level frame)
    assert!(frame.parent_frame().is_none());

    browser.close().await?;
    Ok(())
}

// ============================================================================
// frame.get_by_* combined test
// ============================================================================

// ============================================================================
// frame.evaluate_handle()
// ============================================================================

/// frame.evaluate_handle() evaluates a JS expression and returns a JSHandle/ElementHandle
#[tokio::test]
async fn test_frame_evaluate_handle() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<html><body><h1>Hello</h1></body></html>",
        None,
    )
    .await?;

    let frame = page.main_frame().await?;

    // evaluate_handle returns an Arc<ElementHandle> — the handle to document.body
    let handle = frame.evaluate_handle("document.body").await?;

    // The handle should be non-null (we just verify we get back a valid ElementHandle)
    // We can verify by taking a screenshot of the element (proves the handle is usable)
    let _screenshot = handle.screenshot(None).await?;
    assert!(
        !_screenshot.is_empty(),
        "ElementHandle screenshot should return bytes"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// frame.child_frames()
// ============================================================================

/// Main frame has no child frames on a simple page
#[tokio::test]
async fn test_frame_child_frames_empty_on_simple_page() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(
        "data:text/html,<html><body><p>No iframes here</p></body></html>",
        None,
    )
    .await?;

    let frame = page.main_frame().await?;

    // A simple page with no iframes should have zero child frames
    let children = frame.child_frames();
    assert!(
        children.is_empty(),
        "Main frame on a simple page should have no child frames, got {}",
        children.len()
    );

    browser.close().await?;
    Ok(())
}

/// Main frame child_frames() returns child frames when iframes are present
#[tokio::test]
async fn test_frame_child_frames_with_iframes() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // iframe-test.html has 2 iframes (/iframe-content.html and /iframe-content2.html)
    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    // Wait for iframes to load
    page.wait_for_load_state(None).await?;
    // Give iframes extra time to fully initialize as Frame objects in the protocol
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let frame = page.main_frame().await?;
    let children = frame.child_frames();

    assert!(
        !children.is_empty(),
        "Main frame should have child frames when iframes are present"
    );
    assert_eq!(
        children.len(),
        2,
        "iframe-test.html has 2 iframes, got {}",
        children.len()
    );

    // Each child frame should have a non-empty URL (pointing to the iframe content)
    for child in &children {
        let url = child.url();
        assert!(!url.is_empty(), "Child frame URL should not be empty");
        assert!(
            url.contains("iframe-content") || url.contains("about:blank"),
            "Child frame URL '{}' should contain iframe-content",
            url
        );
    }

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Child frames have parent_frame() pointing back to main frame
#[tokio::test(flavor = "multi_thread")]
async fn test_frame_child_frames_parent_reference() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await?;

    page.wait_for_load_state(None).await?;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let frame = page.main_frame().await?;
    let children = frame.child_frames();

    // Verify child frames report the main frame as their parent
    for child in &children {
        let parent = child.parent_frame();
        assert!(parent.is_some(), "Child frame should have a parent frame");
        assert_eq!(
            parent.unwrap().guid(),
            frame.guid(),
            "Child frame's parent_frame() GUID should match the main frame's GUID"
        );
    }

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Frame get_by_* methods work on main frame
#[tokio::test]
async fn test_frame_get_by_methods() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // Navigate to a page with various elements
    page.goto(
        "data:text/html,<html><body>\
            <input type='text' placeholder='Enter email' data-testid='email-input' />\
            <label for='name'>Full Name</label>\
            <input id='name' type='text' />\
            <img src='data:,' alt='Logo Image' />\
            <span title='Tooltip Text'>item</span>\
            <button role='button'>Submit</button>\
        </body></html>",
        None,
    )
    .await?;

    let frame = page.main_frame().await?;

    // get_by_placeholder
    let by_placeholder = frame.get_by_placeholder("Enter email", true);
    assert_eq!(by_placeholder.count().await?, 1);

    // get_by_label
    let by_label = frame.get_by_label("Full Name", true);
    assert_eq!(by_label.count().await?, 1);

    // get_by_alt_text
    let by_alt = frame.get_by_alt_text("Logo Image", true);
    assert_eq!(by_alt.count().await?, 1);

    // get_by_title
    let by_title = frame.get_by_title("Tooltip Text", true);
    assert_eq!(by_title.count().await?, 1);

    // get_by_test_id
    let by_test_id = frame.get_by_test_id("email-input");
    assert_eq!(by_test_id.count().await?, 1);

    // get_by_role
    use playwright_rs::protocol::locator::AriaRole;
    let by_role = frame.get_by_role(AriaRole::Button, None);
    assert_eq!(by_role.count().await?, 1);

    browser.close().await?;
    server.shutdown();
    Ok(())
}
