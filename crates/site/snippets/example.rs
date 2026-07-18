use playwright_rs::Playwright;

let pw = Playwright::launch().await?;
let browser = pw.chromium().launch().await?;
let page = browser.new_page().await?;
page.goto("https://example.com", None).await?;

let heading = page.locator("h1");
assert_eq!(heading.text_content().await?, Some("Example Domain".into()));

browser.close().await?;
