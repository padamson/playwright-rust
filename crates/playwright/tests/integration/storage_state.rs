use playwright_rs::protocol::{Cookie, StorageState};

#[tokio::test]
async fn test_storage_state_retrieve() {
    let (_pw, browser, context) = crate::common::setup_context().await;
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
    let cookie = Cookie::new("my_cookie", "cookie_value").domain("example.com");
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

#[tokio::test]
async fn test_set_storage_state() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let state =
        StorageState::default().cookies(vec![Cookie::new("session", "abc123").domain("localhost")]);
    context
        .set_storage_state(state)
        .await
        .expect("set_storage_state should succeed");

    let state = context
        .storage_state()
        .await
        .expect("storage_state should succeed");
    assert!(
        state
            .cookies
            .iter()
            .any(|c| c.name == "session" && c.value == "abc123"),
        "Expected cookie 'session=abc123' to be present after set_storage_state"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_set_storage_state_replaces_existing() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    // Add an initial cookie via add_cookies
    context
        .add_cookies(&[Cookie::new("old_cookie", "old_value").domain("example.com")])
        .await
        .expect("add_cookies should succeed");

    // Now replace the storage state with a new one (different domain)
    let new_state = StorageState::default().cookies(vec![
        Cookie::new("new_cookie", "new_value").domain("example.com"),
    ]);
    context
        .set_storage_state(new_state)
        .await
        .expect("set_storage_state should succeed");

    let state = context
        .storage_state()
        .await
        .expect("storage_state should succeed");
    assert!(
        state
            .cookies
            .iter()
            .any(|c| c.name == "new_cookie" && c.value == "new_value"),
        "Expected 'new_cookie' after set_storage_state"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_set_storage_state_with_origins() {
    use playwright_rs::protocol::{LocalStorageItem, Origin};

    let (_pw, browser, context) = crate::common::setup_context().await;

    let state = StorageState::default().origins(vec![Origin::new(
        "https://example.com",
        vec![LocalStorageItem::new("key1", "value1")],
    )]);
    context
        .set_storage_state(state)
        .await
        .expect("set_storage_state with origins should succeed");

    browser.close().await.expect("Failed to close browser");
}
