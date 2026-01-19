// Example: WebSocket Interception
//
// This example demonstrates how to intercept and inspect WebSocket connections
// and messages.
//
// To run this example:
// cargo run --example websocket

use playwright_rs::protocol::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let context = browser.new_context().await?;
    let page = context.new_page().await?;

    // Subscribe to the "websocket" event
    // This handler triggers whenever the page opens a new WebSocket connection
    page.on_websocket(|ws| {
        println!("WebSocket opened: {}", ws.url());

        // Clone ws to move into async block
        let ws_clone = ws.clone();

        Box::pin(async move {
            // Subscribe to "framereceived" events (messages from server)
            ws_clone
                .on_frame_received(|payload| {
                    Box::pin(async move {
                        println!("Create Frame Received: {}", payload);
                        Ok(())
                    })
                })
                .await?;

            // Subscribe to "framesent" events (messages from client)
            ws_clone
                .on_frame_sent(|payload| {
                    Box::pin(async move {
                        println!("Client Frame Sent: {}", payload);
                        Ok(())
                    })
                })
                .await?;

            // Subscribe to "close" event
            ws_clone
                .on_close(|_| {
                    Box::pin(async move {
                        println!("WebSocket closed");
                        Ok(())
                    })
                })
                .await?;

            Ok(())
        })
    })
    .await?;

    println!("Navigating to websocket.org echo test...");
    // Using a public echo test page
    page.goto("https://websocket.org/echo.html", None).await?;

    // In a real scenario, you might interact with the page to trigger WS messages
    // For this example, we just wait a bit to let the page load and potentially connect
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // You can also inspect the page to trigger actions
    // page.click("button:has-text('Connect')").await?;
    // page.click("button:has-text('Send')").await?;

    println!("Closing browser...");
    browser.close().await?;

    Ok(())
}
