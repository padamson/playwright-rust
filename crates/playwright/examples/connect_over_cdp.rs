// Connect over CDP example - Connect to a remote Chrome via Chrome DevTools Protocol
//
// Shows: Connecting to a Chrome instance running in Docker (or elsewhere)
// with remote debugging enabled, then performing browser automation.
//
// ## Prerequisites
//
// 1. Install the Playwright driver (needed locally to manage the CDP connection):
//
//    ```bash
//    npx playwright@1.59.1 install chromium
//    ```
//
// 2. Start a Chrome instance with remote debugging enabled. The easiest way
//    is using Docker:
//
//    ```bash
//    docker run -d --rm --name chrome-cdp -p 9222:9222 \
//      zenika/alpine-chrome \
//      --no-sandbox \
//      --remote-debugging-address=0.0.0.0 \
//      --remote-debugging-port=9222
//    ```
//
//    Or using browserless:
//
//    ```bash
//    docker run -d --rm --name chrome-cdp -p 9222:3000 \
//      ghcr.io/browserless/chromium
//    ```
//
// 3. Run this example:
//
//    ```bash
//    cargo run --package playwright-rs --example connect_over_cdp
//    ```
//
// 4. Clean up:
//
//    ```bash
//    docker stop chrome-cdp
//    ```
//
// ## How it works
//
// `connect_over_cdp` tells the local Playwright driver to connect to the
// remote Chrome's CDP endpoint. The driver handles all protocol translation.
// This is the same architecture used by playwright-python, playwright-java,
// and playwright-dotnet — a local Playwright driver is always required.
//
// ## Customizing the endpoint
//
// Set the CDP_ENDPOINT environment variable to connect to a different host:
//
//    ```bash
//    CDP_ENDPOINT=http://192.168.1.100:9222 cargo run --package playwright-rs --example connect_over_cdp
//    ```

use playwright_rs::{ConnectOverCdpOptions, Playwright, expect};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get CDP endpoint from environment or use default
    let endpoint = std::env::var("CDP_ENDPOINT").unwrap_or_else(|_| "http://localhost:9222".into());

    println!("Connecting to Chrome CDP endpoint: {}", endpoint);

    // Launch local Playwright driver (required for protocol management)
    let playwright = Playwright::launch().await?;

    // Connect to the remote Chrome via CDP (Chromium only)
    let options = ConnectOverCdpOptions::new().timeout(10000.0); // 10s timeout
    let browser = playwright
        .chromium()
        .connect_over_cdp(&endpoint, Some(options))
        .await?;

    println!("Connected! Browser version: {}", browser.version());

    // Create a page and navigate
    let page = browser.new_page().await?;
    page.goto("https://example.com", None).await?;

    // Assert heading text using expect API
    let heading = page.locator("h1").await;
    expect(heading).to_have_text("Example Domain").await?;
    println!("Heading assertion passed");

    // Find the "Learn more" link using get_by_text
    let link = page.get_by_text("Learn more", false).await;
    expect(link.clone()).to_be_visible().await?;
    println!("Link is visible");

    // Click the link — navigates to IANA
    link.click(None).await?;
    let url_after_click = page.url();
    println!("Navigated to: {}", url_after_click);
    assert!(
        url_after_click.contains("iana.org"),
        "Expected IANA URL, got: {}",
        url_after_click
    );

    // Navigate back to example.com
    page.goto("https://example.com", None).await?;
    let heading = page.locator("h1").await;
    expect(heading).to_have_text("Example Domain").await?;
    println!("Back navigation assertion passed");

    // Cleanup
    browser.close().await?;

    println!("All assertions passed!");
    Ok(())
}
