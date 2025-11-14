// Actions example - Element interactions
//
// Shows: click, fill, check, select, file upload
// Note: Uses Google search to demonstrate real interactions

use playwright_rs::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Navigate to Google
    page.goto("https://www.google.com", None).await?;

    // Find search input and interact
    let search = page.locator("textarea[name=q]").await;

    // Click to focus
    search.click(None).await?;

    // Fill text
    search.fill("Playwright Rust", None).await?;

    // Verify value
    let value = search.input_value(None).await?;
    assert_eq!(value, "Playwright Rust");
    println!("Search input contains: {}", value);

    // Clear and refill
    search.clear(None).await?;
    search.fill("Rust browser automation", None).await?;

    // Press Enter to search
    search.press("Enter", None).await?;

    // Wait for results (URL changes)
    // In real code, you'd use wait_for_url or assertions
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    println!("Search completed, URL: {}", page.url());

    browser.close().await?;
    Ok(())
}
