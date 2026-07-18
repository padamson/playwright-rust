page.locator("[data-lang='Java']").click(None).await?;

expect(page.locator("[data-lang='Java']"))
    .to_have_attribute("aria-selected", "true")
    .await?;
expect(page.locator("#comparison"))
    .to_contain_text("Playwright.create()")
    .await?;
