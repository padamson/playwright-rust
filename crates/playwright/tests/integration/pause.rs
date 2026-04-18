use crate::test_server::TestServer;

#[tokio::test]
async fn test_pause_headless() {
    let _server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // In headless mode (default), pause() should have no effect and return immediately
    // This verifies that calling it doesn't crash the bindings
    page.pause().await.expect("Failed to pause");

    browser.close().await.expect("Failed to close browser");
}
