// Browser lifecycle example - Launch browsers, manage contexts
//
// Shows: Multiple browser types, contexts, pages, proper cleanup order

use playwright_rs::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;

    // Launch different browser types
    let chromium = playwright.chromium().launch().await?;
    let firefox = playwright.firefox().launch().await?;
    let webkit = playwright.webkit().launch().await?;

    println!("Launched browsers:");
    println!("  Chromium: {} {}", chromium.name(), chromium.version());
    println!("  Firefox: {} {}", firefox.name(), firefox.version());
    println!("  WebKit: {} {}", webkit.name(), webkit.version());

    // Create isolated browser contexts (like incognito windows)
    let context1 = chromium.new_context().await?;
    let context2 = chromium.new_context().await?;

    // Each context can have multiple pages
    let page1 = context1.new_page().await?;
    let page2 = context2.new_page().await?;

    // Navigate to different sites in each context
    page1.goto("https://example.com", None).await?;
    page2.goto("https://www.rust-lang.org", None).await?;

    println!("\nNavigated pages:");
    println!("  Context 1: {}", page1.url());
    println!("  Context 2: {}", page2.url());

    // Cleanup order: pages → contexts → browsers
    page1.close().await?;
    page2.close().await?;
    context1.close().await?;
    context2.close().await?;
    chromium.close().await?;
    firefox.close().await?;
    webkit.close().await?;

    println!("\nAll browsers closed");

    Ok(())
}
