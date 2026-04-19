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

#[tokio::test]
async fn test_set_storage_state() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let state = StorageState {
        cookies: vec![Cookie {
            name: "session".to_string(),
            value: "abc123".to_string(),
            domain: "localhost".to_string(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: None,
        }],
        origins: vec![],
    };
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
        .add_cookies(&[Cookie {
            name: "old_cookie".to_string(),
            value: "old_value".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: None,
        }])
        .await
        .expect("add_cookies should succeed");

    // Now replace the storage state with a new one (different domain)
    let new_state = StorageState {
        cookies: vec![Cookie {
            name: "new_cookie".to_string(),
            value: "new_value".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: None,
        }],
        origins: vec![],
    };
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

    let state = StorageState {
        cookies: vec![],
        origins: vec![Origin {
            origin: "https://example.com".to_string(),
            local_storage: vec![LocalStorageItem {
                name: "key1".to_string(),
                value: "value1".to_string(),
            }],
        }],
    };
    context
        .set_storage_state(state)
        .await
        .expect("set_storage_state with origins should succeed");

    browser.close().await.expect("Failed to close browser");
}
