// Visual regression testing with to_have_screenshot()
//
// Demonstrates baseline management, diff detection, and assertion options.
//
// Run with:
// cargo run --package playwright-rs --example visual_regression

use playwright_rs::protocol::Playwright;
use playwright_rs::{Animations, ScreenshotAssertionOptions, expect, expect_page};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let baseline_dir = std::env::temp_dir().join("playwright-visual-regression");
    // Start fresh each run so baselines don't carry over
    if baseline_dir.exists() {
        std::fs::remove_dir_all(&baseline_dir)?;
    }
    std::fs::create_dir_all(&baseline_dir)?;

    // --- Example 1: Create a baseline ---
    page.set_content(
        r#"<body style="margin:0;background:white;font-family:monospace">
            <div style="padding:30px">
                <h1 style="color:navy;margin:0">Welcome to Acme Corp</h1>
                <p style="color:#333;font-size:18px">Your trusted partner since 1999</p>
                <div id="hero" style="width:300px;height:80px;background:blue;color:white;
                    display:flex;align-items:center;justify-content:center;font-size:24px;
                    border-radius:8px;margin-top:16px">
                    Get Started
                </div>
                <p id="status" style="color:green;margin-top:16px">● All systems operational</p>
            </div>
        </body>"#,
        None,
    )
    .await?;

    let locator = page.locator("body > div");
    expect(locator)
        .to_have_screenshot(baseline_dir.join("hero.png"), None)
        .await?;
    println!("✓ Baseline created (blue button, 'Get Started', green status)");

    // --- Example 2: Page-level screenshot with mask ---
    let status_locator = page.locator("#status");
    let options = ScreenshotAssertionOptions::builder()
        .mask(vec![status_locator])
        .animations(Animations::Disabled)
        .build();

    expect_page(&page)
        .to_have_screenshot(baseline_dir.join("full-page.png"), Some(options))
        .await?;
    println!("✓ Page screenshot with masked status indicator");

    // Clean up mask overlays injected into the DOM
    page.evaluate_expression(
        "document.querySelectorAll('[data-playwright-mask]').forEach(el => el.remove())",
    )
    .await?;

    // --- Example 3: Tolerance for minor differences ---
    let options = ScreenshotAssertionOptions::builder()
        .max_diff_pixels(100)
        .threshold(0.3)
        .build();

    let locator = page.locator("body > div");
    expect(locator)
        .to_have_screenshot(baseline_dir.join("hero.png"), Some(options))
        .await?;
    println!("✓ Matched with tolerance (max_diff_pixels: 100, threshold: 0.3)");

    // --- Example 4: Simulate a regression and see the diff ---
    // Someone changed the button color, text, and status — a real regression!
    page.set_content(
        r#"<body style="margin:0;background:white;font-family:monospace">
            <div style="padding:30px">
                <h1 style="color:navy;margin:0">Welcome to Acme Corp</h1>
                <p style="color:#333;font-size:18px">Your trusted partner since 1999</p>
                <div id="hero" style="width:300px;height:80px;background:red;color:white;
                    display:flex;align-items:center;justify-content:center;font-size:24px;
                    border-radius:8px;margin-top:16px">
                    Buy Now!!!
                </div>
                <p id="status" style="color:red;margin-top:16px">● 3 incidents reported</p>
            </div>
        </body>"#,
        None,
    )
    .await?;

    let locator = page.locator("body > div");
    let result = expect(locator)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_have_screenshot(baseline_dir.join("hero.png"), None)
        .await;

    match result {
        Ok(_) => println!("  (screenshots matched unexpectedly)"),
        Err(e) => {
            println!("✓ Regression detected!");
            let msg = e.to_string();
            if let Some(first_line) = msg.lines().next() {
                println!("  {}", first_line);
            }
        }
    }

    let actual_path = baseline_dir.join("hero-actual.png");
    let diff_path = baseline_dir.join("hero-diff.png");
    if actual_path.exists() && diff_path.exists() {
        println!("\n  Files generated:");
        println!("  - hero.png        (baseline: blue 'Get Started' button)");
        println!("  - hero-actual.png (actual:   red 'Buy Now!!!' button)");
        println!("  - hero-diff.png   (diff:     red pixels show what changed)");
    }

    // --- Example 5: Accept the change by updating the baseline ---
    let options = ScreenshotAssertionOptions::builder()
        .update_snapshots(true)
        .build();

    let locator = page.locator("body > div");
    expect(locator)
        .to_have_screenshot(baseline_dir.join("hero.png"), Some(options))
        .await?;
    println!("\n✓ Baseline updated to accept the new design");

    // Cleanup
    browser.close().await?;
    println!("\n📂 Open the output folder to inspect the images:");
    println!("  open {}", baseline_dir.display());
    Ok(())
}
