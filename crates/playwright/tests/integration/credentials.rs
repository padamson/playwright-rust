use playwright_rs::CredentialsGetOptions;

#[tokio::test]
async fn test_virtual_credentials_crud() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let creds = context.credentials();
    creds.install().await.expect("install authenticator");

    // No credentials registered yet.
    assert!(creds.get(None).await.expect("get").is_empty());

    let created = creds
        .create("example.com", None)
        .await
        .expect("create credential");
    assert_eq!(created.rp_id, "example.com");
    assert!(!created.id.is_empty(), "authenticator should assign an id");

    let all = creds.get(None).await.expect("get all");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, created.id);

    // Filter by relying party.
    let filtered = creds
        .get(Some(CredentialsGetOptions::default().rp_id("example.com")))
        .await
        .expect("get filtered");
    assert_eq!(filtered.len(), 1);

    let none = creds
        .get(Some(CredentialsGetOptions::default().rp_id("other.test")))
        .await
        .expect("get filtered miss");
    assert!(none.is_empty(), "no credentials for other.test");

    creds.delete(&created.id).await.expect("delete");
    assert!(creds.get(None).await.expect("get after delete").is_empty());

    browser.close().await.expect("close");
}
