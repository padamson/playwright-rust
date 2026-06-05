let opts = ScreenshotOptions::builder()
    .mask(vec![page.locator("#hero-badges img").await])
    .mask_color("#ce422b")
    .build();
page.locator("#hero").await.screenshot(Some(opts)).await?;
