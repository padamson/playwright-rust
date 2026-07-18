let opts = ScreenshotOptions::builder()
    .mask(vec![page.locator("#hero-badges img")])
    .mask_color("#ce422b")
    .build();
page.locator("#hero").screenshot(Some(opts)).await?;
