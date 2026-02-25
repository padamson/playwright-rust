// Locators example - Find and query elements
//
// Shows: Locator API, chaining, nested locators, element queries, get_by_* methods

use playwright_rs::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto("https://example.com", None).await?;

    // Basic locator - find and query element
    let heading = page.locator("h1").await;
    let heading_text = heading.text_content().await?;
    let is_visible = heading.is_visible().await?;
    println!("Heading: {:?} (visible: {})", heading_text, is_visible);

    // Count matching elements
    let paragraphs = page.locator("p").await;
    let count = paragraphs.count().await?;
    println!("Found {} paragraphs", count);

    // Locator chaining - first, last, nth
    let first_para = paragraphs.first();
    let first_text = first_para.inner_text().await?;
    println!(
        "First paragraph: {}",
        first_text.lines().next().unwrap_or("")
    );

    // Nested locators - scope search within element
    let body = page.locator("body").await;
    let links = body.locator("a");
    let link_count = links.count().await?;
    println!("Found {} links in body", link_count);

    if link_count > 0 {
        let first_link = links.first();
        let link_text = first_link.text_content().await?;
        println!("First link: {:?}", link_text);
    }

    // get_by_text - find elements by their text content
    let heading = page.get_by_text("Example Domain", false).await;
    let is_visible = heading.is_visible().await?;
    println!("Found heading by text (visible: {})", is_visible);

    // get_by_text with exact matching
    let domain = page.get_by_text("example", false).await;
    let count = domain.count().await?;
    println!(
        "Found {} elements containing 'example' (case-insensitive)",
        count
    );

    // Chained get_by_text - search within a container
    let link_by_text = body.get_by_text("Learn more", false);
    let link_count = link_by_text.count().await?;
    println!("Found {} elements with 'Learn more' in body", link_count);

    // get_by_title - find elements by title attribute
    let titled = page.get_by_title("Example Domain", false).await;
    println!(
        "Found {} elements with title 'Example Domain'",
        titled.count().await?
    );

    browser.close().await?;
    Ok(())
}
