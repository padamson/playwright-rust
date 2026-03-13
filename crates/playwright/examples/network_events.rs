// Example: Network Event Listening
//
// Demonstrates how to listen for network events on a Page:
// - page.on_request() - fires when a request is issued
// - page.on_response() - fires when a response is received
// - page.on_request_finished() - fires when a request finishes successfully
// - page.on_request_failed() - fires when a request fails
//
// To run this example:
// cargo run --example network_events

use playwright_rs::protocol::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Network Events Example ===\n");

    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Listen for outgoing requests
    page.on_request(|request| async move {
        println!(">> {} {}", request.method(), request.url());
        Ok(())
    })
    .await?;

    // Listen for incoming responses
    page.on_response(|response| async move {
        println!(
            "<< {} {} {}",
            response.status(),
            response.status_text(),
            response.url()
        );
        Ok(())
    })
    .await?;

    // Listen for completed requests
    page.on_request_finished(|request| async move {
        println!("-- finished: {}", request.url());
        Ok(())
    })
    .await?;

    // Listen for failed requests
    page.on_request_failed(|request| async move {
        println!("!! failed: {}", request.url());
        Ok(())
    })
    .await?;

    println!("Navigating to example.com...\n");
    page.goto("https://example.com", None).await?;

    // Small delay to let async event dispatching complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    println!("\nClosing browser...");
    browser.close().await?;

    Ok(())
}
