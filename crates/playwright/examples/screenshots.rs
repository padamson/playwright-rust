// Screenshot examples demonstrating various screenshot options
//
// Run with:
//   cargo run --package playwright-rs --example screenshots

use playwright_rs::protocol::{
    Animations, Caret, Playwright, Scale, ScreenshotClip, ScreenshotOptions, ScreenshotType,
};

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
    let heading = page.locator("h1");
    let element_bytes = heading.screenshot(None).await?;
    println!("✓ Element screenshot: {} bytes", element_bytes.len());

    // Example 6: Save screenshot to file
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("playwright_example_screenshot.png");
    page.screenshot_to_file(&file_path, None).await?;
    println!("✓ Screenshot saved to: {}", file_path.display());

    // Example 7: Stable capture — freeze CSS animations, hide the text caret,
    // and capture at device-pixel scale (all new in Playwright 1.60).
    let stable = ScreenshotOptions::builder()
        .animations(Animations::Disabled)
        .caret(Caret::Hide)
        .scale(Scale::Device)
        .build();
    let stable_bytes = page.screenshot(Some(stable)).await?;
    println!(
        "✓ Stable screenshot (animations off, caret hidden, device scale): {} bytes",
        stable_bytes.len()
    );

    // Example 8: Inject CSS before capturing (e.g. hide flaky/dynamic elements).
    let styled = ScreenshotOptions::builder()
        .style("h1 { visibility: hidden; }")
        .build();
    let styled_bytes = page.screenshot(Some(styled)).await?;
    println!(
        "✓ Screenshot with injected CSS: {} bytes",
        styled_bytes.len()
    );

    // Example 9: Mask elements — overpaint matched locators with a solid box to
    // redact dynamic or sensitive content.
    let masked = ScreenshotOptions::builder()
        .mask(vec![page.locator("h1")])
        .mask_color("#FF00FF")
        .build();
    let masked_bytes = page.screenshot(Some(masked)).await?;
    println!(
        "✓ Masked screenshot (h1 redacted): {} bytes",
        masked_bytes.len()
    );

    // Cleanup
    browser.close().await?;
    println!("\n✅ All screenshot examples completed!");

    Ok(())
}
