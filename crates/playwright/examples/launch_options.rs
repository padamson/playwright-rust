// Launch options example - Browser launch configurations
//
// Shows: Headless mode, custom args, slow motion, DevTools
//
// This example demonstrates various browser launch configurations using LaunchOptions.
// All options are passed to the underlying Playwright server and forwarded to the browser.
//
// Note: App mode (--app flag) is not included here because it requires
// launchPersistentContext() which is not yet implemented. See issue #9.

use playwright_rs::Playwright;
use playwright_rs::api::LaunchOptions;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;

    // Example 1: Headless mode with custom args
    println!("Example 1: Headless with custom args");
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

    // Example 2: Slow motion - Useful for debugging and demonstrations
    println!("Example 2: Slow motion");
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

    // Example 3: DevTools auto-open
    println!("Example 3: DevTools");
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

    // Example 4: Window size (via args)
    println!("Example 4: Custom window size");
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
    println!("\nTip: You can combine options - e.g., slow motion + custom args");
    println!("See LaunchOptions struct for all available options.");

    Ok(())
}
