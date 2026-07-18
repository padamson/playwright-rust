// Every card shows its own snippet, actually highlighted.
for (id, token) in cards {
    expect(page.locator(id))
        .to_contain_text(token)
        .await?;
    let colored = page
        .locator(&format!("{id} span[style*='color']"))
        .count()
        .await?;
    assert!(colored > 0); // syntect emitted colored spans
}
