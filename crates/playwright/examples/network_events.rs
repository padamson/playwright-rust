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
    let response = page
        .goto("https://example.com", None)
        .await?
        .expect("Expected a response");

    // Small delay to let async event dispatching complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // --- Response body access ---

    println!("\n=== Response Body Access ===\n");

    // Get response body as text
    let body_text = response.text().await?;
    println!(
        "Response text (first 80 chars): {}...",
        &body_text[..body_text.len().min(80)]
    );

    // Get response body as raw bytes
    let body_bytes = response.body().await?;
    println!("Response body size: {} bytes", body_bytes.len());

    // Get individual header value
    if let Some(content_type) = response.header_value("content-type").await? {
        println!("Content-Type: {}", content_type);
    }

    // Get all headers as name/value pairs (preserves duplicates)
    let headers = response.headers_array().await?;
    println!("Response has {} header entries", headers.len());

    println!("\nClosing browser...");
    browser.close().await?;

    Ok(())
}
