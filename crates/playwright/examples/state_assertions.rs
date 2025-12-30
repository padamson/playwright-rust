// State assertions examples demonstrating enabled, checked, and editable assertions
//
// Run with:
// PLAYWRIGHT_DRIVER_PATH=./drivers/playwright-1.56.1-mac-arm64 \
//     cargo run --package playwright --example state_assertions

use playwright_rs::expect;
use playwright_rs::protocol::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Launch Playwright and browser
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Navigate to a page
    page.goto("https://example.com", None).await?;
    println!("✓ Navigated to example.com");

    // Create test elements with different states
    page.evaluate_expression(
        r#"
        // Create enabled button
        const enabledBtn = document.createElement('button');
        enabledBtn.id = 'enabled-btn';
        enabledBtn.textContent = 'Click me';
        document.body.appendChild(enabledBtn);

        // Create disabled button
        const disabledBtn = document.createElement('button');
        disabledBtn.id = 'disabled-btn';
        disabledBtn.textContent = 'Disabled';
        disabledBtn.disabled = true;
        document.body.appendChild(disabledBtn);

        // Create checked checkbox
        const checkedBox = document.createElement('input');
        checkedBox.type = 'checkbox';
        checkedBox.id = 'checked-box';
        checkedBox.checked = true;
        document.body.appendChild(checkedBox);

        // Create unchecked checkbox
        const uncheckedBox = document.createElement('input');
        uncheckedBox.type = 'checkbox';
        uncheckedBox.id = 'unchecked-box';
        document.body.appendChild(uncheckedBox);

        // Create editable input
        const editableInput = document.createElement('input');
        editableInput.type = 'text';
        editableInput.id = 'editable-input';
        editableInput.value = 'Edit me';
        document.body.appendChild(editableInput);

        // Create readonly input (not editable)
        const readonlyInput = document.createElement('input');
        readonlyInput.type = 'text';
        readonlyInput.id = 'readonly-input';
        readonlyInput.value = 'Read only';
        readonlyInput.readOnly = true;
        document.body.appendChild(readonlyInput);
        "#,
    )
    .await?;

    // Example 1: Assert button is enabled
    let enabled_btn = page.locator("#enabled-btn").await;
    expect(enabled_btn.clone()).to_be_enabled().await?;
    println!("✓ Enabled button is enabled");

    // Example 2: Assert button is disabled
    let disabled_btn = page.locator("#disabled-btn").await;
    expect(disabled_btn.clone()).to_be_disabled().await?;
    println!("✓ Disabled button is disabled");

    // Example 3: Negation - assert enabled button is NOT disabled
    expect(enabled_btn.clone()).not().to_be_disabled().await?;
    println!("✓ Enabled button is NOT disabled (negation)");

    // Example 4: Assert checkbox is checked
    let checked_box = page.locator("#checked-box").await;
    expect(checked_box.clone()).to_be_checked().await?;
    println!("✓ Checked checkbox is checked");

    // Example 5: Assert checkbox is unchecked
    let unchecked_box = page.locator("#unchecked-box").await;
    expect(unchecked_box.clone()).to_be_unchecked().await?;
    println!("✓ Unchecked checkbox is unchecked");

    // Example 6: Negation - assert checked box is NOT unchecked
    expect(checked_box).not().to_be_unchecked().await?;
    println!("✓ Checked checkbox is NOT unchecked (negation)");

    // Example 7: Assert input is editable
    let editable_input = page.locator("#editable-input").await;
    expect(editable_input.clone()).to_be_editable().await?;
    println!("✓ Editable input is editable");

    // Example 8: Assert readonly input is NOT editable
    let readonly_input = page.locator("#readonly-input").await;
    expect(readonly_input.clone())
        .not()
        .to_be_editable()
        .await?;
    println!("✓ Readonly input is NOT editable (negation)");

    // Example 9: Auto-retry demonstration - button becomes enabled after delay
    page.evaluate_expression(
        r#"
        const delayedBtn = document.createElement('button');
        delayedBtn.id = 'delayed-btn';
        delayedBtn.textContent = 'Will be enabled';
        delayedBtn.disabled = true;
        document.body.appendChild(delayedBtn);

        setTimeout(() => {
            delayedBtn.disabled = false;
        }, 1000);
        "#,
    )
    .await?;

    // This will auto-retry for up to 5 seconds, waiting for button to become enabled
    let delayed_btn = page.locator("#delayed-btn").await;
    expect(delayed_btn).to_be_enabled().await?;
    println!("✓ Delayed button became enabled (auto-retry)");

    // Example 10: Auto-retry - checkbox becomes checked after delay
    page.evaluate_expression(
        r#"
        const delayedBox = document.createElement('input');
        delayedBox.type = 'checkbox';
        delayedBox.id = 'delayed-box';
        document.body.appendChild(delayedBox);

        setTimeout(() => {
            delayedBox.checked = true;
        }, 500);
        "#,
    )
    .await?;

    let delayed_box = page.locator("#delayed-box").await;
    expect(delayed_box).to_be_checked().await?;
    println!("✓ Delayed checkbox became checked (auto-retry)");

    // Example 11: Custom timeout
    page.evaluate_expression(
        r#"
        const slowBtn = document.createElement('button');
        slowBtn.id = 'slow-btn';
        slowBtn.disabled = true;
        document.body.appendChild(slowBtn);

        setTimeout(() => {
            slowBtn.disabled = false;
        }, 2000);
        "#,
    )
    .await?;

    let slow_btn = page.locator("#slow-btn").await;
    expect(slow_btn)
        .with_timeout(std::time::Duration::from_secs(10))
        .to_be_enabled()
        .await?;
    println!("✓ Slow button became enabled (with 10s timeout)");

    // Example 12: Timeout error handling
    page.evaluate_expression(
        r#"
        const foreverDisabled = document.createElement('button');
        foreverDisabled.id = 'forever-disabled';
        foreverDisabled.disabled = true;
        document.body.appendChild(foreverDisabled);
        "#,
    )
    .await?;

    let forever_disabled = page.locator("#forever-disabled").await;
    let result = expect(forever_disabled)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_be_enabled()
        .await;

    match result {
        Ok(_) => println!("❌ Should have timed out"),
        Err(e) => println!("✓ Assertion timed out as expected: {}", e),
    }

    // Cleanup
    browser.close().await?;
    println!("\n✅ All state assertion examples completed!");

    Ok(())
}
