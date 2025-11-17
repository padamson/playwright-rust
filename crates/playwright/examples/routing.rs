// Network routing examples demonstrating request interception
//
// Run with:
// PLAYWRIGHT_DRIVER_PATH=./drivers/playwright-1.56.1-mac-arm64 \
//     cargo run --package playwright --example routing

use playwright_rs::protocol::{FulfillOptions, Playwright};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Launch Playwright and browser
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    println!("🌐 Network Routing Examples\n");

    // Example 1: Block all image requests
    println!("Example 1: Block all images");
    page.route("**/*.{png,jpg,jpeg,gif}", |route| async move {
        println!("  ❌ Blocked image: {}", route.request().url());
        route.abort(None).await
    })
    .await?;

    page.goto("https://example.com", None).await?;
    println!("  ✓ Page loaded (images blocked)\n");

    // Example 2: Continue all requests (passthrough)
    println!("Example 2: Passthrough all requests");
    let page2 = browser.new_page().await?;
    page2
        .route("**/*", |route| async move {
            println!("  → Allowing: {}", route.request().url());
            route.continue_(None).await
        })
        .await?;

    page2.goto("https://example.com", None).await?;
    println!("  ✓ All requests allowed\n");

    // Example 3: Conditional routing based on URL
    println!("Example 3: Conditional abort based on URL");
    let page3 = browser.new_page().await?;
    page3
        .route("**/*", |route| async move {
            let request = route.request();
            let url = request.url();
            if url.contains("analytics") || url.contains("tracking") {
                println!("  ❌ Blocked analytics: {}", url);
                route.abort(None).await
            } else {
                println!("  ✓ Allowed: {}", url);
                route.continue_(None).await
            }
        })
        .await?;

    page3.goto("https://example.com", None).await?;
    println!("  ✓ Conditional routing complete\n");

    // Example 4: Multiple route handlers with priority
    println!("Example 4: Multiple handlers (last registered wins)");
    let page4 = browser.new_page().await?;

    // First handler - blocks CSS
    page4
        .route("**/*.css", |route| async move {
            println!("  ❌ CSS handler: {}", route.request().url());
            route.abort(None).await
        })
        .await?;

    // Second handler - blocks JavaScript
    page4
        .route("**/*.js", |route| async move {
            println!("  ❌ JS handler: {}", route.request().url());
            route.abort(None).await
        })
        .await?;

    // Third handler - allows HTML
    page4
        .route("**/*.html", |route| async move {
            println!("  ✓ HTML handler: {}", route.request().url());
            route.continue_(None).await
        })
        .await?;

    page4.goto("https://example.com", None).await?;
    println!("  ✓ Multiple handlers configured\n");

    // Example 5: Abort with specific error code
    println!("Example 5: Abort with error code");
    let page5 = browser.new_page().await?;
    page5
        .route("**/*.png", |route| async move {
            println!("  ❌ Access denied: {}", route.request().url());
            route.abort(Some("accessdenied")).await
        })
        .await?;

    page5.goto("https://example.com", None).await?;
    println!("  ✓ Requests aborted with error code\n");

    // Example 6: Glob pattern matching
    println!("Example 6: Glob pattern examples");
    let page6 = browser.new_page().await?;

    // Block all static assets
    page6
        .route(
            "**/*.{css,js,png,jpg,jpeg,gif,svg,woff,woff2}",
            |route| async move {
                println!("  ❌ Blocked static: {}", route.request().url());
                route.abort(None).await
            },
        )
        .await?;

    page6.goto("https://example.com", None).await?;
    println!("  ✓ Glob patterns matched\n");

    // Example 7: Access request data in handler
    println!("Example 7: Inspect request data");
    let page7 = browser.new_page().await?;
    page7
        .route("**/*", |route| async move {
            let request = route.request();
            println!("  📋 {} {}", request.method(), request.url());
            route.continue_(None).await
        })
        .await?;

    page7.goto("https://example.com", None).await?;
    println!("  ✓ Request data inspected\n");

    // Example 8: Mock responses with fulfill()
    println!("Example 8: Mock API with custom response");
    let page8 = browser.new_page().await?;
    page8
        .route("**/api/data", |route| async move {
            let options = FulfillOptions::builder()
                .status(200)
                .body_string("Mocked response")
                .content_type("text/plain")
                .build();
            println!("  🎭 Mocked API: {}", route.request().url());
            route.fulfill(Some(options)).await
        })
        .await?;

    page8.goto("https://example.com", None).await?;
    println!("  ✓ Custom text response mocked\n");

    // Example 9: Mock JSON responses
    println!("Example 9: Mock JSON API response");
    let page9 = browser.new_page().await?;
    page9
        .route("**/api/users", |route| async move {
            let data = json!({
                "users": [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"}
                ],
                "total": 2
            });
            let options = FulfillOptions::builder()
                .json(&data)
                .expect("Failed to serialize JSON")
                .build();
            println!("  🎭 Mocked JSON: {}", route.request().url());
            route.fulfill(Some(options)).await
        })
        .await?;

    page9.goto("https://example.com", None).await?;
    println!("  ✓ JSON response mocked\n");

    // Example 10: Custom status codes
    println!("Example 10: Mock with custom status code");
    let page10 = browser.new_page().await?;
    page10
        .route("**/api/error", |route| async move {
            let options = FulfillOptions::builder()
                .status(404)
                .body_string("Not Found")
                .content_type("text/plain")
                .build();
            println!("  🎭 Mocked 404: {}", route.request().url());
            route.fulfill(Some(options)).await
        })
        .await?;

    page10.goto("https://example.com", None).await?;
    println!("  ✓ Custom status code mocked\n");

    // Example 11: Custom headers
    println!("Example 11: Mock with custom headers");
    let page11 = browser.new_page().await?;
    page11
        .route("**/api/headers", |route| async move {
            let mut headers = std::collections::HashMap::new();
            headers.insert("X-Custom-Header".to_string(), "CustomValue".to_string());
            headers.insert("X-API-Version".to_string(), "v2".to_string());

            let options = FulfillOptions::builder()
                .headers(headers)
                .body_string("Response with custom headers")
                .content_type("text/plain")
                .build();
            println!("  🎭 Mocked with headers: {}", route.request().url());
            route.fulfill(Some(options)).await
        })
        .await?;

    page11.goto("https://example.com", None).await?;
    println!("  ✓ Custom headers mocked\n");

    // Cleanup
    browser.close().await?;
    println!("✅ All routing examples completed!");

    Ok(())
}
