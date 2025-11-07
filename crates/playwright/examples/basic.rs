// Basic example of using Playwright in Rust
//
// This example demonstrates:
// - Launching Playwright
// - Accessing browser types (Chromium, Firefox, WebKit)
// - Launching a browser
// - Creating a page
// - Navigating to a URL
// - Getting page information (title, URL)
// - Proper cleanup

use playwright::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    println!("🚀 Launching Playwright...");

    // Launch Playwright (connects to Playwright server)
    let playwright = Playwright::launch().await?;

    println!("✅ Playwright launched successfully!\n");

    // Access browser types
    println!("📦 Available browser types:");
    println!("   • Chromium: {}", playwright.chromium().executable_path());
    println!("   • Firefox:  {}", playwright.firefox().executable_path());
    println!("   • WebKit:   {}", playwright.webkit().executable_path());

    // Launch a browser
    println!("\n🌐 Launching Chromium...");
    let browser = playwright.chromium().launch().await?;
    println!(
        "✅ Browser launched: {} version {}",
        browser.name(),
        browser.version()
    );

    // Create a page
    println!("\n📄 Creating page...");
    let page = browser.new_page().await?;
    println!("✅ Page created at: {}", page.url());

    // Navigate to a URL
    println!("\n🔗 Navigating to example.com...");
    let response = page.goto("https://example.com", None).await?;
    println!("✅ Navigation successful!");
    println!("   • Status: {}", response.status());
    println!("   • URL: {}", response.url());

    // Get page information
    let title = page.title().await?;
    println!("\n📋 Page information:");
    println!("   • Title: {}", title);
    println!("   • Current URL: {}", page.url());

    // Cleanup
    println!("\n🧹 Cleaning up...");
    page.close().await?;
    browser.close().await?;

    println!("\n🎉 Example complete!");

    Ok(())
}
