// Take the backend down. The app copes: only the current build is
// offered, and the "go to latest" nudge is gone.
*backend.lock().unwrap() = None;
page.reload(None).await?;
expect(page.locator("#version-select option"))
    .to_have_count(1)
    .await?;
expect(page.locator("#version-switcher a"))
    .to_have_count(0)
    .await?;
