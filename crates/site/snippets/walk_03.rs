page.locator("#feature-cross-browser [data-lang='Firefox']")
    .click(None)
    .await?;

expect(page.locator("#feature-cross-browser"))
    .to_contain_text("firefox")
    .await?;
