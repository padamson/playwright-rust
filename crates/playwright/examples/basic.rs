// Basic example of using Playwright in Rust
//
// This example demonstrates Phase 1 & 2 functionality:
// - Launching Playwright
// - Accessing browser types (Chromium, Firefox, WebKit)
// - Launching a browser
// - Creating a page
// - Proper cleanup
//
// Note: Navigation and interaction will be implemented in Phase 3.

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

    // Launch a browser (Phase 2)
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
    println!("✅ Page created (URL: {})", page.url());

    // Cleanup
    println!("\n🧹 Cleaning up...");
    page.close().await?;
    browser.close().await?;

    println!("\n🎉 Phases 1 & 2 complete!");
    println!("   (Navigation and interaction coming in Phase 3)");

    Ok(())
}
