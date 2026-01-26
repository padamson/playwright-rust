// Responsive Testing Example
//
// Demonstrates using page.set_viewport_size() to test responsive layouts
// at different screen sizes.

use playwright_rs::protocol::{Playwright, Viewport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Navigate to a responsive website
    page.goto("https://example.com", None).await?;

    println!("Testing responsive layout at different viewport sizes...\n");

    // Test mobile viewport (iPhone SE)
    let mobile = Viewport {
        width: 375,
        height: 667,
    };
    page.set_viewport_size(mobile).await?;

    let mobile_width: u32 = page.evaluate("window.innerWidth", None::<&()>).await?;
    let mobile_height: u32 = page.evaluate("window.innerHeight", None::<&()>).await?;
    println!("📱 Mobile (iPhone SE): {}x{}", mobile_width, mobile_height);

    // Test tablet viewport (iPad portrait)
    let tablet = Viewport {
        width: 768,
        height: 1024,
    };
    page.set_viewport_size(tablet).await?;

    let tablet_width: u32 = page.evaluate("window.innerWidth", None::<&()>).await?;
    let tablet_height: u32 = page.evaluate("window.innerHeight", None::<&()>).await?;
    println!("📱 Tablet (iPad): {}x{}", tablet_width, tablet_height);

    // Test desktop viewport (Full HD)
    let desktop = Viewport {
        width: 1920,
        height: 1080,
    };
    page.set_viewport_size(desktop).await?;

    let desktop_width: u32 = page.evaluate("window.innerWidth", None::<&()>).await?;
    let desktop_height: u32 = page.evaluate("window.innerHeight", None::<&()>).await?;
    println!(
        "🖥️  Desktop (Full HD): {}x{}",
        desktop_width, desktop_height
    );

    // Test ultrawide viewport (2K)
    let ultrawide = Viewport {
        width: 2560,
        height: 1440,
    };
    page.set_viewport_size(ultrawide).await?;

    let ultrawide_width: u32 = page.evaluate("window.innerWidth", None::<&()>).await?;
    let ultrawide_height: u32 = page.evaluate("window.innerHeight", None::<&()>).await?;
    println!(
        "🖥️  Ultrawide (2K): {}x{}",
        ultrawide_width, ultrawide_height
    );

    println!("\n✅ Successfully tested responsive layouts at 4 different viewport sizes");

    browser.close().await?;
    Ok(())
}
