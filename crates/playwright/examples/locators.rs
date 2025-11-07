// Locators example - Finding and querying elements
//
// This example demonstrates:
// - Creating locators to find elements
// - Querying element text and state
// - Locator chaining (first, last, nth)
// - Nested locators
// - Cross-browser compatibility

use playwright::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎭 Playwright Locators Example\n");

    // Launch Playwright
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Navigate to a page
    println!("🔗 Navigating to example.com...");
    page.goto("https://example.com", None).await?;
    println!("✅ Page loaded\n");

    // Basic locator - find heading
    println!("📍 Finding elements with locators:");
    let heading = page.locator("h1").await;
    let heading_text = heading.text_content().await?;
    println!("   • Heading: {:?}", heading_text);

    // Check element visibility
    let is_visible = heading.is_visible().await?;
    println!("   • Heading visible: {}", is_visible);

    // Count elements
    println!("\n📊 Counting elements:");
    let paragraphs = page.locator("p").await;
    let count = paragraphs.count().await?;
    println!("   • Found {} paragraph(s)", count);

    // Locator chaining - get first paragraph
    println!("\n🔗 Locator chaining:");
    let first_para = paragraphs.first();
    let first_text = first_para.inner_text().await?;
    println!(
        "   • First paragraph: {}",
        first_text.lines().next().unwrap_or("")
    );

    // Nested locators - find links within body
    println!("\n🎯 Nested locators:");
    let body = page.locator("body").await;
    let links = body.locator("a");
    let link_count = links.count().await?;
    println!("   • Found {} link(s) in body", link_count);

    if link_count > 0 {
        let first_link = links.first();
        let link_text = first_link.text_content().await?;
        println!("   • First link text: {:?}", link_text);
    }

    // Query element properties
    println!("\n🔍 Querying element properties:");
    let div = page.locator("div").await;
    if div.count().await? > 0 {
        let inner_html = div.first().inner_html().await?;
        println!(
            "   • First div HTML length: {} characters",
            inner_html.len()
        );
    }

    // Cleanup
    println!("\n🧹 Cleaning up...");
    page.close().await?;
    browser.close().await?;

    println!("\n🎉 Example complete!");

    Ok(())
}
