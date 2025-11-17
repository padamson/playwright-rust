// Screenshot examples demonstrating various screenshot options
//
// Run with:
// PLAYWRIGHT_DRIVER_PATH=./drivers/playwright-1.56.1-mac-arm64 \
//     cargo run --package playwright --example screenshots

use playwright_rs::protocol::{Playwright, ScreenshotClip, ScreenshotOptions, ScreenshotType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Launch Playwright and browser
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Navigate to a page
    page.goto("https://example.com", None).await?;
    println!("✓ Navigated to example.com");

    // Example 1: Basic PNG screenshot (default)
    let png_bytes = page.screenshot(None).await?;
    println!("✓ PNG screenshot: {} bytes", png_bytes.len());

    // Example 2: JPEG screenshot with quality
    let jpeg_options = ScreenshotOptions::builder()
        .screenshot_type(ScreenshotType::Jpeg)
        .quality(80)
        .build();
    let jpeg_bytes = page.screenshot(Some(jpeg_options)).await?;
    println!("✓ JPEG screenshot (quality 80): {} bytes", jpeg_bytes.len());

    // Example 3: Full-page screenshot
    let fullpage_options = ScreenshotOptions::builder().full_page(true).build();
    let fullpage_bytes = page.screenshot(Some(fullpage_options)).await?;
    println!("✓ Full-page screenshot: {} bytes", fullpage_bytes.len());

    // Example 4: Clip region screenshot
    let clip = ScreenshotClip {
        x: 0.0,
        y: 0.0,
        width: 400.0,
        height: 300.0,
    };
    let clip_options = ScreenshotOptions::builder().clip(clip).build();
    let clip_bytes = page.screenshot(Some(clip_options)).await?;
    println!(
        "✓ Clip region screenshot (400x300): {} bytes",
        clip_bytes.len()
    );

    // Example 5: Element screenshot
    let heading = page.locator("h1").await;
    let element_bytes = heading.screenshot(None).await?;
    println!("✓ Element screenshot: {} bytes", element_bytes.len());

    // Example 6: Save screenshot to file
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("playwright_example_screenshot.png");
    page.screenshot_to_file(&file_path, None).await?;
    println!("✓ Screenshot saved to: {}", file_path.display());

    // Cleanup
    browser.close().await?;
    println!("\n✅ All screenshot examples completed!");

    Ok(())
}
