use playwright_rs::protocol::{Cookie, StorageState, StorageStateOptions};

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
        .storage_state(None)
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
        .storage_state(None)
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
        .storage_state(None)
        .await
        .expect("storage_state should succeed");
    assert!(
        state
            .cookies
            .iter()
            .any(|c| c.name == "new_cookie" && c.value == "new_value"),
        "Expected 'new_cookie' after set_storage_state"
    );
    // Replace means replace: the pre-existing cookie must be gone, not
    // merged alongside the new one.
    assert!(
        !state.cookies.iter().any(|c| c.name == "old_cookie"),
        "set_storage_state should replace existing cookies, but 'old_cookie' survived"
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

#[tokio::test]
async fn test_storage_state_captures_webauthn_credentials() {
    let (_pw, browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await.expect("Failed to create page");
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    context
        .credentials()
        .install()
        .await
        .expect("install virtual authenticator");
    let created = context
        .credentials()
        .create("example.com", None)
        .await
        .expect("create passkey");

    // Passkeys are opt-in: a state captured without the flag must look exactly
    // as it did before 1.62, so existing saved states stay meaningful.
    let without = context
        .storage_state(None)
        .await
        .expect("storage_state without credentials");
    assert!(
        without.credentials.is_none(),
        "credentials should be absent unless asked for"
    );

    let with = context
        .storage_state(StorageStateOptions::default().credentials(true))
        .await
        .expect("storage_state with credentials");
    let creds = with
        .credentials
        .clone()
        .expect("credentials should be present when requested");
    assert!(
        creds.iter().any(|c| c.id == created.id),
        "the created passkey should appear in the captured state"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_set_storage_state_restores_webauthn_credentials() {
    let (_pw, browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await.expect("Failed to create page");
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    context
        .credentials()
        .install()
        .await
        .expect("install virtual authenticator");
    let created = context
        .credentials()
        .create("example.com", None)
        .await
        .expect("create passkey");
    let saved = context
        .storage_state(StorageStateOptions::default().credentials(true))
        .await
        .expect("capture state with credentials");

    // A fresh context starts with no passkeys. Restoring the saved state has
    // to bring them back, which is what the client-side restore could not do:
    // it replayed cookies and localStorage only.
    let fresh = browser
        .new_context()
        .await
        .expect("Failed to create second context");
    fresh
        .credentials()
        .install()
        .await
        .expect("install virtual authenticator in fresh context");
    assert!(
        fresh
            .credentials()
            .get(None)
            .await
            .expect("list passkeys")
            .is_empty(),
        "a fresh context should hold no passkeys"
    );

    fresh
        .set_storage_state(saved)
        .await
        .expect("restore state carrying credentials");

    let restored = fresh
        .credentials()
        .get(None)
        .await
        .expect("list passkeys after restore");
    assert!(
        restored.iter().any(|c| c.id == created.id),
        "the saved passkey should be restored into the new context"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_storage_state_round_trips_indexed_db() {
    let (_pw, browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await.expect("Failed to create page");
    page.goto("https://example.com", None)
        .await
        .expect("Failed to navigate");

    // Write a value through IndexedDB and wait for the transaction to land.
    let _: bool = page
        .evaluate(
            "() => new Promise(resolve => { \
               const open = indexedDB.open('db', 1); \
               open.onupgradeneeded = () => open.result.createObjectStore('kv'); \
               open.onsuccess = () => { \
                 const tx = open.result.transaction('kv', 'readwrite'); \
                 tx.objectStore('kv').put('value-42', 'key'); \
                 tx.oncomplete = () => resolve(true); \
               }; \
             })",
            None::<&()>,
        )
        .await
        .expect("seed IndexedDB");

    let saved = context
        .storage_state(StorageStateOptions::default().indexed_db(true))
        .await
        .expect("capture state with IndexedDB");
    let origin = saved
        .origins
        .iter()
        .find(|o| o.origin.contains("example.com"))
        .expect("captured state should carry the visited origin");
    assert!(
        origin.indexed_db.is_some(),
        "indexed_db(true) should capture the IndexedDB payload, got none"
    );

    // Restore into a fresh context and read the value back through the API.
    let fresh = browser
        .new_context()
        .await
        .expect("Failed to create second context");
    fresh
        .set_storage_state(saved)
        .await
        .expect("restore state carrying IndexedDB");
    let page2 = fresh.new_page().await.expect("new page in fresh context");
    page2
        .goto("https://example.com", None)
        .await
        .expect("navigate in fresh context");
    let value: String = page2
        .evaluate(
            "() => new Promise(resolve => { \
               const open = indexedDB.open('db', 1); \
               open.onsuccess = () => { \
                 const tx = open.result.transaction('kv', 'readonly'); \
                 const get = tx.objectStore('kv').get('key'); \
                 get.onsuccess = () => resolve(get.result ?? ''); \
               }; \
             })",
            None::<&()>,
        )
        .await
        .expect("read IndexedDB in fresh context");
    assert_eq!(value, "value-42", "IndexedDB should survive the round-trip");

    browser.close().await.expect("Failed to close browser");
}

/// The origin private file system round-trips through storage state, the same
/// way IndexedDB does: opt in when capturing, and it is restored verbatim.
#[tokio::test]
async fn opfs_round_trips_through_storage_state() -> Result<(), Box<dyn std::error::Error>> {
    let server = crate::test_server::TestServer::start().await;
    let (_pw, browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await?;
    page.goto(&format!("{}/", server.url()), None).await?;

    // Write a file into the origin private file system.
    page.evaluate_value(
        r#"navigator.storage.getDirectory()
             .then(root => root.getFileHandle('note.txt', { create: true }))
             .then(handle => handle.createWritable())
             .then(w => w.write('remembered').then(() => w.close()))
             .then(() => 'written')"#,
    )
    .await?;

    let state = context
        .storage_state(playwright_rs::protocol::StorageStateOptions::default().opfs(true))
        .await?;
    assert!(
        state.origins.iter().any(|o| o.opfs.is_some()),
        "capturing with opfs(true) should carry the origin's files"
    );

    // Restore into a fresh context and read the file back.
    let restored = browser.new_context().await?;
    restored.set_storage_state(state).await?;
    let restored_page = restored.new_page().await?;
    restored_page
        .goto(&format!("{}/", server.url()), None)
        .await?;
    let text = restored_page
        .evaluate_value(
            r#"navigator.storage.getDirectory()
                 .then(root => root.getFileHandle('note.txt'))
                 .then(handle => handle.getFile())
                 .then(file => file.text())"#,
        )
        .await?;
    assert_eq!(text, "remembered");

    restored.close().await?;
    context.close().await?;
    browser.close().await?;
    server.shutdown();
    Ok(())
}
