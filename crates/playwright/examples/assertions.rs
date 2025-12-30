// Assertions examples demonstrating auto-retry assertions
//
// Run with:
// PLAYWRIGHT_DRIVER_PATH=./drivers/playwright-1.56.1-mac-arm64 \
//     cargo run --package playwright --example assertions

use playwright_rs::expect;
use playwright_rs::protocol::Playwright;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Launch Playwright and browser
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Navigate to a page with elements
    page.goto("https://example.com", None).await?;
    println!("✓ Navigated to example.com");

    // Example 1: Assert element is visible
    let heading = page.locator("h1").await;
    expect(heading.clone()).to_be_visible().await?;
    println!("✓ Heading is visible");

    // Example 2: Assert element is hidden
    // (nonexistent elements are considered hidden)
    let dialog = page.locator("#dialog").await;
    expect(dialog.clone()).to_be_hidden().await?;
    println!("✓ Dialog is hidden");

    // Example 3: Negation - assert element is NOT visible
    expect(dialog.clone()).not().to_be_visible().await?;
    println!("✓ Dialog is NOT visible (negation)");

    // Example 4: Custom timeout
    // Assertions default to 5 seconds, but you can customize
    expect(heading.clone())
        .with_timeout(Duration::from_secs(10))
        .to_be_visible()
        .await?;
    println!("✓ Heading is visible (with 10s timeout)");

    // Example 5: Auto-retry demonstration
    // Inject a delayed element using evaluate()
    page.evaluate_expression(
        r#"
        const delayed = document.createElement('div');
        delayed.id = 'delayed-element';
        delayed.textContent = 'I will appear!';
        delayed.style.display = 'none';
        document.body.appendChild(delayed);

        setTimeout(() => {
            delayed.style.display = 'block';
        }, 1000);
        "#,
    )
    .await?;

    // This will auto-retry for up to 5 seconds, waiting for element to become visible
    let delayed = page.locator("#delayed-element").await;
    expect(delayed).to_be_visible().await?;
    println!("✓ Delayed element became visible (auto-retry)");

    // Example 6: Text assertions
    expect(heading.clone())
        .to_have_text("Example Domain")
        .await?;
    println!("✓ Heading has exact text");

    expect(heading.clone()).to_contain_text("Example").await?;
    println!("✓ Heading contains substring");

    // Example 7: Regex pattern matching
    expect(heading.clone())
        .to_have_text_regex(r"Example.*")
        .await?;
    println!("✓ Heading matches regex pattern");

    // Example 8: Input value assertions
    // Inject an input with a value
    page.evaluate_expression(
        r#"
        const input = document.createElement('input');
        input.id = 'email-input';
        input.value = 'user@example.com';
        document.body.appendChild(input);
        "#,
    )
    .await?;

    let email_input = page.locator("#email-input").await;
    expect(email_input.clone())
        .to_have_value("user@example.com")
        .await?;
    println!("✓ Input has expected value");

    expect(email_input)
        .to_have_value_regex(r".*@example\.com")
        .await?;
    println!("✓ Input value matches regex");

    // Example 9: Timeout error handling
    // Create element that stays hidden
    page.evaluate_expression(
        r#"
        const hidden = document.createElement('div');
        hidden.id = 'hidden-element';
        hidden.style.display = 'none';
        document.body.appendChild(hidden);
        "#,
    )
    .await?;

    let hidden = page.locator("#hidden-element").await;
    let result = expect(hidden)
        .with_timeout(Duration::from_millis(500))
        .to_be_visible()
        .await;

    match result {
        Ok(_) => println!("❌ Should have timed out"),
        Err(e) => println!("✓ Assertion timed out as expected: {}", e),
    }

    // Cleanup
    browser.close().await?;
    println!("\n✅ All assertion examples completed!");

    Ok(())
}
