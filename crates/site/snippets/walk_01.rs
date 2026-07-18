let page = context.new_page().await?;
page.goto(url, None).await?;

// Locators auto-wait for the WASM app to mount. No sleeps.
expect(page.locator("#hero-title"))
    .to_have_text("Playwright for Rust")
    .await?;
