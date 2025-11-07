// Browser lifecycle example
//
// This example demonstrates Phase 2 functionality:
// - Launching browsers
// - Creating contexts and pages
// - Proper cleanup/teardown
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

    // Launch Chromium browser
    println!("🌐 Launching Chromium...");
    let browser = playwright.chromium().launch().await?;
    println!(
        "✅ Browser launched: {} version {}\n",
        browser.name(),
        browser.version()
    );

    // Create a browser context (isolated session)
    println!("📦 Creating browser context...");
    let context = browser.new_context().await?;
    println!("✅ Context created\n");

    // Create pages in the context
    println!("📄 Creating pages...");
    let page1 = context.new_page().await?;
    println!("   • Page 1 created (URL: {})", page1.url());

    let page2 = context.new_page().await?;
    println!("   • Page 2 created (URL: {})\n", page2.url());

    // Alternatively, use browser.new_page() for convenience
    println!("📄 Creating page via browser.new_page() convenience method...");
    let page3 = browser.new_page().await?;
    println!("   • Page 3 created (URL: {})\n", page3.url());

    // Cleanup (in reverse order of creation)
    println!("🧹 Cleaning up...");

    page3.close().await?;
    println!("   • Page 3 closed");

    page2.close().await?;
    println!("   • Page 2 closed");

    page1.close().await?;
    println!("   • Page 1 closed");

    context.close().await?;
    println!("   • Context closed");

    browser.close().await?;
    println!("   • Browser closed");

    println!("\n🎉 Phase 2 complete! Browser lifecycle management working.");
    println!("   (Navigation and interaction coming in Phase 3)");

    Ok(())
}
