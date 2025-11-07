// Basic example of using Playwright in Rust
//
// This example demonstrates the Phase 1 functionality:
// - Launching Playwright
// - Accessing browser types (Chromium, Firefox, WebKit)
//
// Note: Phase 1 only provides access to browser type objects.
// Actual browser launching will be implemented in Phase 2.

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

    println!("\n🎉 Phase 1 complete! Browser types are accessible.");
    println!("   (Browser launching will be implemented in Phase 2)");

    Ok(())
}
