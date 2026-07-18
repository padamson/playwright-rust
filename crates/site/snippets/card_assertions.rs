expect(page.locator("#status"))
    .to_have_text("Ready")
    .await?;
