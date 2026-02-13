//! CDP Connection Example
//!
//! Demonstrates connecting to an existing browser via Chrome DevTools Protocol,
//! discovering existing contexts/pages, and interacting with them.
//!
//! Usage:
//!   1. Launch Chrome with: chrome --remote-debugging-port=9223
//!   2. Navigate to a page (e.g., https://www.wikipedia.org)
//!   3. Run: cargo run --example test_cdp
//!
//! Or just run this example directly - it launches Chrome automatically.

use playwright_rs::Playwright;
use playwright_rs::server::channel_owner::ChannelOwner;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const CDP_PORT: u16 = 9223;

fn launch_chrome(port: u16, url: &str) -> Result<Child, String> {
    let chrome_path = if cfg!(target_os = "macos") {
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    } else if cfg!(target_os = "windows") {
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
    } else {
        "/usr/bin/google-chrome"
    };

    let temp_dir = std::env::temp_dir().join(format!("playwright-cdp-test-{}", port));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).ok();

    Command::new(chrome_path)
        .arg(format!("--remote-debugging-port={}", port))
        .arg(format!("--user-data-dir={}", temp_dir.to_string_lossy()))
        .arg("--window-size=1280,720")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-extensions")
        .arg("--disable-default-apps")
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to launch Chrome: {}", e))
}

async fn wait_for_cdp(port: u16) -> Result<(), String> {
    for _ in 0..30 {
        if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err("CDP endpoint not ready".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CDP Connection Example ===\n");

    // Launch Chrome with Wikipedia pre-loaded
    println!("Launching Chrome with Wikipedia...");
    let mut chrome = launch_chrome(CDP_PORT, "https://www.wikipedia.org")?;
    wait_for_cdp(CDP_PORT).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    println!("Chrome ready!\n");

    // Connect via CDP
    let playwright = Playwright::launch().await?;
    let browser = playwright
        .chromium()
        .connect_over_cdp(&format!("http://localhost:{}", CDP_PORT), None)
        .await?;

    // Discover existing contexts and pages
    let contexts = browser.contexts();
    println!("Found {} context(s)", contexts.len());

    let ctx = &contexts[0];
    let pages = ctx.pages();
    println!("Found {} page(s)", pages.len());
    for (i, p) in pages.iter().enumerate() {
        println!("  Page {}: GUID={}, URL={}", i, p.guid(), p.url());
    }

    let page = pages.last().unwrap();
    println!("\nUsing page: {}", page.guid());

    // Activate the page and set viewport
    page.bring_to_front().await?;
    page.set_viewport_size(playwright_rs::Viewport {
        width: 1280,
        height: 720,
    })
    .await?;

    // Get real URL via JS (page.url() may be stale for CDP-attached pages)
    let url: String = page.evaluate("() => window.location.href", None::<&()>).await?;
    println!("URL: {}", url);

    // Get page title
    let title = page.title().await?;
    println!("Title: {}", title);

    // Take screenshot
    let screenshot = page.screenshot(None).await?;
    std::fs::write("/tmp/cdp_example.png", &screenshot)?;
    println!("Screenshot: {} bytes -> /tmp/cdp_example.png", screenshot.len());

    // Click on search input using mouse coordinates
    println!("\nClicking search input...");
    // Get the search input position via JS
    let pos: serde_json::Value = page.evaluate(
        r#"() => {
            const el = document.querySelector('input[name=search]');
            if (!el) return { x: 640, y: 360 };
            const rect = el.getBoundingClientRect();
            return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
        }"#,
        None::<&()>,
    ).await?;
    let cx = pos["x"].as_f64().unwrap_or(640.0) as i32;
    let cy = pos["y"].as_f64().unwrap_or(360.0) as i32;
    println!("Search input at ({}, {})", cx, cy);
    page.mouse().click(cx, cy, None).await?;

    // Type a search query
    println!("Typing 'Rust programming language'...");
    page.keyboard().type_text("Rust programming language", None).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify typing worked
    let value: String = page.evaluate(
        "() => document.querySelector('input[name=search]')?.value || ''",
        None::<&()>,
    ).await?;
    println!("Search input value: '{}'", value);

    // Take final screenshot
    let screenshot2 = page.screenshot(None).await?;
    std::fs::write("/tmp/cdp_example_after.png", &screenshot2)?;
    println!("Final screenshot -> /tmp/cdp_example_after.png");

    if value.contains("Rust") {
        println!("\n✅ CDP connection and interaction successful!");
    } else {
        println!("\n❌ Interaction did not work as expected");
    }

    // Cleanup
    chrome.kill().ok();
    chrome.wait().ok();
    println!("Done!");
    Ok(())
}
