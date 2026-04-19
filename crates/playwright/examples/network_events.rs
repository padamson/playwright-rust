// Example: Network Event Listening and Request/Response Inspection
//
// Demonstrates how to listen for network events on a Page:
// - page.on_request() / on_response() / on_request_finished() / on_request_failed()
// - Response body access: body(), text(), header_value(), headers_array()
// - Back-references: response.request(), response.frame(), request.frame()
// - Server info: response.server_addr(), response.security_details()
// - Request completion: request.response(), request.sizes()
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
    // Set up a deterministic waiter so we know event handlers have fired
    // before we inspect the response (preferred over sleeps).
    let waiter = page.expect_event("response", Some(10_000.0)).await?;
    let response = page
        .goto("https://example.com", None)
        .await?
        .expect("Expected a response");
    waiter.wait().await?;

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

    // --- Back-references ---

    println!("\n=== Back-references ===\n");

    // Navigate from response -> request -> frame
    if let Some(request) = response.request() {
        println!("Response request: {} {}", request.method(), request.url());
        if let Some(frame) = request.frame() {
            println!("Request frame URL: {}", frame.url());
        }
    }
    if let Some(frame) = response.frame() {
        println!("Response frame URL: {}", frame.url());
    }

    // --- Server address & security details ---

    println!("\n=== Server Info ===\n");

    if let Some(addr) = response.server_addr().await? {
        println!("Server address: {}:{}", addr.ip_address, addr.port);
    }

    // HTTPS connections provide TLS security details
    match response.security_details().await? {
        Some(details) => {
            println!("TLS protocol: {:?}", details.protocol);
            println!("Certificate issuer: {:?}", details.issuer);
        }
        None => println!("No TLS (HTTP connection)"),
    }

    // --- Request.response() and sizes ---

    println!("\n=== Request -> Response & Sizes ===\n");

    // Use on_request_finished to capture a request. Since expect_event does not
    // cover "request_finished", signal the handler via tokio::sync::Notify for
    // a deterministic wait.
    let page2 = browser.new_page().await?;
    let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let captured_clone = captured.clone();
    let notify = std::sync::Arc::new(tokio::sync::Notify::new());
    let notify_clone = notify.clone();
    page2
        .on_request_finished(move |req| {
            let cap = captured_clone.clone();
            let n = notify_clone.clone();
            async move {
                if req.is_navigation_request() {
                    *cap.lock().unwrap() = Some(req);
                    n.notify_one();
                }
                Ok(())
            }
        })
        .await?;

    page2.goto("https://example.com", None).await?;
    tokio::time::timeout(std::time::Duration::from_secs(10), notify.notified())
        .await
        .expect("request_finished handler did not fire");

    let captured_req = captured.lock().unwrap().take();
    if let Some(req) = captured_req {
        // Get the Response from the Request
        if let Ok(Some(resp)) = req.response().await {
            println!("request.response() status: {}", resp.status());
        }
        // Get resource sizes
        if let Ok(sizes) = req.sizes().await {
            println!("Request headers size: {} bytes", sizes.request_headers_size);
            println!("Response body size: {} bytes", sizes.response_body_size);
        }
    }

    println!("\nClosing browser...");
    browser.close().await?;

    Ok(())
}
