// Launch options example - Browser launch configurations
//
// Shows: App mode, headless mode, custom args, slow motion, DevTools
//
// This example demonstrates various browser launch configurations using LaunchOptions.
// All options are passed to the underlying Playwright server and forwarded to the browser.

use playwright_rs::api::LaunchOptions;
use playwright_rs::Playwright;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;

    // Example 1: App mode - Browser without UI chrome (no address bar, tabs, etc.)
    // This is useful for creating app-like experiences or kiosk modes
    println!("Example 1: App mode");
    println!("Launching Chromium in app mode (minimal UI)...");
    let browser = playwright
        .chromium()
        .launch_with_options(
            LaunchOptions::new()
                .args(vec!["--app=https://example.com".to_string()])
                .headless(false),
        )
        .await?;

    let page = browser.new_page().await?;
    println!("  App mode active - browser has minimal UI");
    println!("  URL: {}", page.url());
    sleep(Duration::from_secs(2)).await;
    browser.close().await?;
    println!();

    // Example 2: Headless mode with custom args
    println!("Example 2: Headless with custom args");
    println!("Launching headless browser with no-sandbox...");
    let browser = playwright
        .chromium()
        .launch_with_options(LaunchOptions::new().headless(true).args(vec![
            "--no-sandbox".to_string(),
            "--disable-setuid-sandbox".to_string(),
        ]))
        .await?;

    let page = browser.new_page().await?;
    page.goto("https://example.com", None).await?;
    println!("  Headless navigation complete");
    println!("  Title: {}", page.title().await?);
    browser.close().await?;
    println!();

    // Example 3: Slow motion - Useful for debugging and demonstrations
    println!("Example 3: Slow motion");
    println!("Launching with 500ms delay between operations...");
    let browser = playwright
        .chromium()
        .launch_with_options(
            LaunchOptions::new().headless(false).slow_mo(500.0), // 500ms delay
        )
        .await?;

    let page = browser.new_page().await?;
    println!("  Watch the slow motion navigation...");
    page.goto("https://example.com", None).await?;
    println!("  Navigation complete (notice the delay)");
    sleep(Duration::from_secs(2)).await;
    browser.close().await?;
    println!();

    // Example 4: DevTools auto-open
    println!("Example 4: DevTools");
    println!("Launching with DevTools panel open...");
    let browser = playwright
        .chromium()
        .launch_with_options(LaunchOptions::new().devtools(true).headless(false))
        .await?;

    let page = browser.new_page().await?;
    page.goto("https://example.com", None).await?;
    println!("  DevTools should be visible in the browser window");
    sleep(Duration::from_secs(3)).await;
    browser.close().await?;
    println!();

    // Example 5: Window size (via args)
    println!("Example 5: Custom window size");
    println!("Launching with custom window dimensions...");
    let browser = playwright
        .chromium()
        .launch_with_options(LaunchOptions::new().headless(false).args(vec![
            "--window-size=800,600".to_string(),
            "--window-position=100,100".to_string(),
        ]))
        .await?;

    let page = browser.new_page().await?;
    page.goto("https://example.com", None).await?;
    println!("  Browser launched at 800x600 resolution");
    sleep(Duration::from_secs(2)).await;
    browser.close().await?;
    println!();

    println!("All examples complete!");
    println!("\nTip: You can combine options - e.g., app mode + slow motion");
    println!("See LaunchOptions struct for all available options.");

    Ok(())
}
