use playwright_rs::expect;

#[tokio::test]
async fn test_to_match_aria_snapshot_basic() {
    let (_playwright, browser, page) = crate::common::setup().await;

    page.set_content("<h1>Hello</h1><button>Click me</button>", None)
        .await
        .expect("Failed to set content");

    let body = page.locator("body").await;
    expect(body)
        .to_match_aria_snapshot("- heading \"Hello\" [level=1]\n- button \"Click me\"")
        .await
        .expect("ARIA snapshot should match heading and button");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_to_match_aria_snapshot_negation() {
    let (_playwright, browser, page) = crate::common::setup().await;

    page.set_content("<h1>Hello</h1><button>Click me</button>", None)
        .await
        .expect("Failed to set content");

    let body = page.locator("body").await;
    // A snapshot that does NOT match should pass with .not()
    expect(body)
        .not()
        .to_match_aria_snapshot("- heading \"Goodbye\" [level=1]")
        .await
        .expect("ARIA snapshot for wrong heading should NOT match");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_to_match_aria_snapshot_mismatch_fails() {
    let (_playwright, browser, page) = crate::common::setup().await;

    page.set_content("<h1>Hello</h1>", None)
        .await
        .expect("Failed to set content");

    let body = page.locator("body").await;
    let result = expect(body)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_match_aria_snapshot("- heading \"Goodbye\" [level=1]")
        .await;

    assert!(
        result.is_err(),
        "Mismatched ARIA snapshot should return an error"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_aria_snapshot_matches_body_locator() {
    let (_playwright, browser, page) = crate::common::setup().await;

    page.set_content("<h1>Hello</h1><button>Click me</button>", None)
        .await
        .expect("Failed to set content");

    // Page::aria_snapshot() should produce the same YAML as the
    // explicit locator("body").aria_snapshot() form it shorthand-wraps.
    let from_page = page
        .aria_snapshot()
        .await
        .expect("Page::aria_snapshot should succeed");
    let from_locator = page
        .locator("body")
        .await
        .aria_snapshot()
        .await
        .expect("Locator::aria_snapshot should succeed");

    assert_eq!(
        from_page, from_locator,
        "Page::aria_snapshot must equal page.locator(\"body\").aria_snapshot()"
    );

    // Sanity-check the snapshot mentions both the heading and the button.
    assert!(
        from_page.contains("Hello"),
        "Snapshot should mention heading text: {from_page}"
    );
    assert!(
        from_page.contains("Click me"),
        "Snapshot should mention button text: {from_page}"
    );

    browser.close().await.expect("Failed to close browser");
}
