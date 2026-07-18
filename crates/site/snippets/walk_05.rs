expect(page.locator("#disclaimer"))
    .to_contain_text("unofficial")
    .await?;
