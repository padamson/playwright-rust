use playwright_rs::protocol::{Cookie, Playwright};
// use tempfile::TempDir; (removed unused)

mod common;

#[tokio::test]
async fn test_storage_state_retrieve() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let chromium = playwright.chromium();
    let browser = chromium.launch().await.expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // 1. Set up initial state (cookies and local storage)
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Set LocalStorage
    // We expect no return value, so use Unit `()` for deserialization
    page.evaluate::<_, ()>("localStorage.setItem('my_key', 'my_value')", None::<&()>)
        .await
        .expect("Failed to set localStorage");

    // Set Cookie using add_cookies
    let cookie = Cookie {
        name: "my_cookie".into(),
        value: "cookie_value".into(),
        domain: "example.com".into(), // Or .example.com
        path: "/".into(),
        expires: -1.0,
        http_only: false,
        secure: false, // example.com might be strict, but we are setting locally
        same_site: None,
    };
    context
        .add_cookies(&[cookie])
        .await
        .expect("Failed to add cookies");

    // 2. Call storage_state()
    let state = context
        .storage_state()
        .await
        .expect("Failed to get storage state");

    // 3. Verify contents
    // Check cookies
    let cookie = state.cookies.iter().find(|c| c.name == "my_cookie");
    assert!(cookie.is_some(), "Cookie not found");

    // Check localStorage
    let origin_state = state
        .origins
        .iter()
        .find(|o| o.origin == "https://example.com");
    assert!(origin_state.is_some(), "Origin state not found");

    if let Some(origin) = origin_state {
        let item = origin.local_storage.iter().find(|i| i.name == "my_key");
        assert!(item.is_some(), "Local storage item not found");
        assert_eq!(item.unwrap().value, "my_value");
    }

    browser.close().await.expect("Failed to close browser");
}
