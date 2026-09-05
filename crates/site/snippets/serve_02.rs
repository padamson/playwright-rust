// The app fetched /versions.json and the router answered from the test's
// JSON: it now offers a release that exists nowhere but in this test.
expect(page.locator("#version-select"))
    .to_contain_text("v9.9.9")
    .await?;
expect(page.locator("#version-switcher a"))
    .to_contain_text("v9.9.9")
    .await?;
