// Basic example - Launch browser, navigate, and get page info
//
// Shows: Playwright initialization, browser launch, navigation, cleanup

use playwright_rs::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Launch Playwright and browser
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Navigate and check response
    let response = page
        .goto("https://example.com", None)
        .await?
        .expect("https://example.com should return a response");
    assert!(response.ok());
    assert_eq!(response.status(), 200);

    // Get page info
    let title = page.title().await?;
    let url = page.url();
    println!("Title: {}", title);
    println!("URL: {}", url);

    // Cleanup
    browser.close().await?;

    Ok(())
}
