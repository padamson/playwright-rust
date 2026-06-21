use crate::test_server::TestServer;

#[tokio::test]
async fn test_local_storage_roundtrip() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // localStorage needs a real (non-opaque) origin.
    page.goto(&format!("{}/keyboard_mouse.html", server.url()), None)
        .await
        .expect("navigate");

    let ls = page.local_storage();

    assert_eq!(ls.get_item("missing").await.expect("get"), None);

    ls.set_item("token", "abc123").await.expect("set");
    assert_eq!(
        ls.get_item("token").await.expect("get"),
        Some("abc123".to_string())
    );

    ls.set_item("user", "ada").await.expect("set");
    let mut items = ls.items().await.expect("items");
    items.sort();
    assert_eq!(
        items,
        vec![
            ("token".to_string(), "abc123".to_string()),
            ("user".to_string(), "ada".to_string()),
        ]
    );

    ls.remove_item("token").await.expect("remove");
    assert_eq!(ls.get_item("token").await.expect("get"), None);

    ls.clear().await.expect("clear");
    assert!(ls.items().await.expect("items").is_empty());

    browser.close().await.expect("close");
    server.shutdown();
}

#[tokio::test]
async fn test_local_and_session_storage_are_independent() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&format!("{}/keyboard_mouse.html", server.url()), None)
        .await
        .expect("navigate");

    page.local_storage()
        .set_item("k", "local")
        .await
        .expect("set local");
    page.session_storage()
        .set_item("k", "session")
        .await
        .expect("set session");

    assert_eq!(
        page.local_storage().get_item("k").await.expect("get"),
        Some("local".to_string())
    );
    assert_eq!(
        page.session_storage().get_item("k").await.expect("get"),
        Some("session".to_string())
    );

    browser.close().await.expect("close");
    server.shutdown();
}
