// Integration tests for WebSocket event handling
//
// Following TDD: Write tests first (Red), then implement (Green)

mod test_server; // Assuming we can link to the shared module or need to copy it?
// Ideally we reuse the existing test_server.rs in tests/

use playwright_rs::protocol::Playwright;
use test_server::TestServer;

mod common;

#[tokio::test]
async fn test_websocket_interception() {
    common::init_tracing();
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

    // Setup WebSocket event handler
    // This API does not exist yet -> RED
    let ws_event_fired = std::sync::Arc::new(tokio::sync::Mutex::new(false));
    let ws_event_fired_clone = ws_event_fired.clone();

    page.on_websocket(move |ws| {
        let fired = ws_event_fired_clone.clone();
        Box::pin(async move {
            *fired.lock().await = true;
            println!("WebSocket opened: {}", ws.url());

            // Verify URL
            assert!(ws.url().contains("ws://"));

            // Listen for frames
            ws.on_frame_sent(|data| {
                Box::pin(async move {
                    println!("Frame sent: {:?}", data);
                    Ok(())
                })
            })
            .await
            .unwrap();

            Ok(())
        })
    })
    .await
    .expect("Failed to register websocket handler");

    // Navigate to a page that opens a WebSocket
    // We need to add a websocket test page to test_server
    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Wait a bit for the connection
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    assert!(
        *ws_event_fired.lock().await,
        "on_websocket handler should have been called"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
