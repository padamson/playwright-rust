// Example: Headless API testing with playwright.request()
//
// Demonstrates the APIRequest / APIRequestContext API for exercising HTTP
// endpoints without launching a browser:
// - playwright.request().new_context(options) — build a request context with
//   optional base_url, extra headers, etc.
// - context.get() / post() / put() / delete() / patch() / head() / fetch()
// - APIResponse accessors: status(), status_text(), ok(), url(), headers()
// - APIResponse body access: body(), text(), json::<T>()
// - context.dispose() — release server resources
// - FetchOptions builder — method, headers, post_data, max_redirects, timeout
//
// Typical uses:
// - Integration-test a REST API without a browser
// - Prime server state (log in, seed data) before running browser tests
// - Assert on response shape/headers/body without UI round-trips
//
// To run:
//   cargo run --package playwright-rs --example api_request

use playwright_rs::protocol::Playwright;
use playwright_rs::{APIRequestContextOptions, FetchOptions};
use serde::Deserialize;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== playwright.request() — headless API testing ===\n");

    let playwright = Playwright::launch().await?;

    // --- Create a context with a base URL ---
    // base_url lets us pass relative paths to get() / post() / etc.
    let ctx = playwright
        .request()
        .new_context(Some(APIRequestContextOptions {
            base_url: Some("https://httpbin.org".to_string()),
            ..Default::default()
        }))
        .await?;

    // --- GET request returning JSON ---
    println!(">> GET /get");
    let response = ctx.get("/get", None).await?;
    println!(
        "   status: {} {}",
        response.status(),
        response.status_text()
    );
    assert!(response.ok());

    // Parse the JSON body into a typed struct
    #[derive(Deserialize, Debug)]
    struct HttpBinGet {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    }

    let body: HttpBinGet = response.json().await?;
    println!("   echoed url: {}", body.url);
    println!("   echoed header count: {}", body.headers.len());

    // --- POST request with body + custom headers ---
    println!("\n>> POST /post with JSON body");
    let mut headers = HashMap::new();
    headers.insert("x-test-header".to_string(), "playwright-rust".to_string());
    headers.insert("content-type".to_string(), "application/json".to_string());

    let opts = FetchOptions::builder()
        .headers(headers)
        .post_data(r#"{"hello":"world"}"#.to_string())
        .build();

    let response = ctx.post("/post", Some(opts)).await?;
    println!("   status: {}", response.status());

    // Read the body as text
    let text = response.text().await?;
    println!("   response length: {} bytes", text.len());

    // --- PUT / DELETE / PATCH / HEAD round out the HTTP verb coverage ---
    println!("\n>> PUT /put");
    let response = ctx.put("/put", None).await?;
    println!("   status: {}", response.status());

    println!("\n>> DELETE /delete");
    let response = ctx.delete("/delete", None).await?;
    println!("   status: {}", response.status());

    println!("\n>> HEAD /get");
    let response = ctx.head("/get", None).await?;
    println!("   status: {}", response.status());
    // HEAD responses have headers but (by spec) no body.
    if let Some(length) = response.headers().get("content-length") {
        println!("   content-length header: {}", length);
    }

    // --- Timeout + max_redirects via FetchOptions ---
    println!("\n>> fetch() with timeout + max_redirects");
    let opts = FetchOptions::builder()
        .method("GET".to_string())
        .max_redirects(5)
        .timeout(10_000.0)
        .build();
    let response = ctx
        .fetch("https://httpbin.org/redirect/2", Some(opts))
        .await?;
    println!("   final status: {}", response.status());
    println!("   final url:    {}", response.url());

    // --- Dispose the context to free server-side resources ---
    ctx.dispose().await?;
    println!("\nDisposed request context.");

    // Shut down the Playwright server
    playwright.shutdown().await?;
    Ok(())
}
