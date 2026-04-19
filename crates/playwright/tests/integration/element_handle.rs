use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;

/// Exercises ElementHandle and query selector methods in a single browser session:
/// query_selector (found), query_selector (not found), query_selector_all,
/// element screenshot, locator screenshot, owner_frame, wait_for_element_state.
#[tokio::test]
async fn test_element_handle_methods() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // query_selector returns Some(ElementHandle) for existing element
    let element = page
        .query_selector("h1")
        .await
        .expect("Failed to query selector");
    assert!(element.is_some(), "Should find h1 element");

    // query_selector returns None for non-existent element
    let element = page
        .query_selector(".does-not-exist")
        .await
        .expect("Failed to query selector");
    assert!(
        element.is_none(),
        "Should return None for non-existent element"
    );

    // query_selector_all returns Vec of ElementHandles
    let elements = page
        .query_selector_all("p")
        .await
        .expect("Failed to query selector all");
    // locators.html has 4 paragraphs
    assert_eq!(elements.len(), 4, "Should find 4 paragraph elements");

    // ElementHandle.screenshot() captures element screenshot
    let element = page
        .query_selector("h1")
        .await
        .expect("Failed to query selector")
        .expect("h1 should exist");
    let bytes = element
        .screenshot(None)
        .await
        .expect("Failed to take element screenshot");
    assert!(!bytes.is_empty(), "Screenshot bytes should not be empty");
    assert_eq!(
        &bytes[0..4],
        &[0x89, 0x50, 0x4E, 0x47],
        "Screenshot should be PNG format"
    );

    // locator.screenshot() delegates to ElementHandle
    let locator = page.locator("h1").await;
    let bytes = locator
        .screenshot(None)
        .await
        .expect("Failed to take locator screenshot");
    assert!(!bytes.is_empty(), "Screenshot bytes should not be empty");
    assert_eq!(
        &bytes[0..4],
        &[0x89, 0x50, 0x4E, 0x47],
        "Screenshot should be PNG format"
    );

    // owner_frame() returns a Frame for an element in the main frame
    let element = page
        .query_selector("h1")
        .await
        .expect("Failed to query selector")
        .expect("h1 should exist");
    let frame = element.owner_frame().await.expect("owner_frame() failed");
    assert!(
        frame.is_some(),
        "owner_frame() should return a Frame for an element in the main frame"
    );

    // wait_for_element_state('visible') resolves immediately for a visible element
    let element = page
        .query_selector("h1")
        .await
        .expect("Failed to query selector")
        .expect("h1 should exist");
    element
        .wait_for_element_state("visible", None)
        .await
        .expect(
            "wait_for_element_state('visible') should resolve immediately for a visible element",
        );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_element_handle_content_frame() {
    let server = TestServer::start().await;
    let (_playwright, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/iframe-test.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let iframe_element = page
        .query_selector("iframe#frame1")
        .await
        .expect("Failed to query selector")
        .expect("iframe#frame1 should exist");

    let frame = iframe_element
        .content_frame()
        .await
        .expect("content_frame() failed");

    assert!(
        frame.is_some(),
        "content_frame() should return a Frame for an iframe element"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// Cross-browser tests

#[tokio::test]
#[ignore]
async fn test_element_handle_screenshot_firefox() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let element = page
        .query_selector("h1")
        .await
        .expect("Failed to query selector")
        .expect("h1 should exist");

    let bytes = element
        .screenshot(None)
        .await
        .expect("Failed to take element screenshot");

    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
#[ignore]
async fn test_element_handle_screenshot_webkit() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let element = page
        .query_selector("h1")
        .await
        .expect("Failed to query selector")
        .expect("h1 should exist");

    let bytes = element
        .screenshot(None)
        .await
        .expect("Failed to take element screenshot");

    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
