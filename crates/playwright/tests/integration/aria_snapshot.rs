use playwright_rs::Error;
use playwright_rs::protocol::{AriaSnapshotMode, AriaSnapshotOptions};
use playwright_rs::{expect, expect_page};

#[tokio::test]
async fn test_to_match_aria_snapshot_basic() {
    let (_playwright, browser, page) = crate::common::setup().await;

    page.set_content("<h1>Hello</h1><button>Click me</button>", None)
        .await
        .expect("Failed to set content");

    let body = page.locator("body");
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

    let body = page.locator("body");
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

    let body = page.locator("body");
    let result = expect(body)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_match_aria_snapshot("- heading \"Goodbye\" [level=1]")
        .await;

    // A genuine mismatch must surface as an assertion variant, never as a
    // generic ProtocolError. The connection layer classifies on the content of
    // the driver's `errorDetails`; the 1.61 driver attaches that object to
    // every `expect` error, so an infrastructure failure (empty details) is a
    // ProtocolError while a real mismatch/timeout populates a field. This pins
    // the end-to-end wire path the error_parsing unit tests can't reach.
    assert!(
        matches!(
            result,
            Err(Error::AssertionFailed(_) | Error::AssertionTimeout(_))
        ),
        "Mismatched ARIA snapshot should be an assertion error, got: {result:?}"
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
        .aria_snapshot(None)
        .await
        .expect("Page::aria_snapshot should succeed");
    let from_locator = page
        .locator("body")
        .aria_snapshot(None)
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

#[tokio::test]
async fn test_aria_snapshot_options_plumb_through() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(
        "<main><h1>Title</h1><nav><a href='#'>Home</a><a href='#'>About</a></nav></main>",
        None,
    )
    .await
    .expect("Failed to set content");

    let body = page.locator("body");

    let default_snapshot = body
        .aria_snapshot(None)
        .await
        .expect("default aria_snapshot should succeed");

    let ai_snapshot = body
        .aria_snapshot(Some(
            AriaSnapshotOptions::default().mode(AriaSnapshotMode::Ai),
        ))
        .await
        .expect("aria_snapshot(mode=Ai) should succeed");

    let depth_snapshot = body
        .aria_snapshot(Some(AriaSnapshotOptions::default().depth(1)))
        .await
        .expect("aria_snapshot(depth=1) should succeed");

    assert!(
        !default_snapshot.is_empty(),
        "default snapshot should be non-empty"
    );
    assert!(!ai_snapshot.is_empty(), "ai snapshot should be non-empty");
    assert!(
        !depth_snapshot.is_empty(),
        "depth-limited snapshot should be non-empty"
    );

    // depth=1 should be shorter than the full default snapshot.
    assert!(
        depth_snapshot.len() <= default_snapshot.len(),
        "depth=1 ({} bytes) should be no longer than default ({} bytes)",
        depth_snapshot.len(),
        default_snapshot.len()
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_to_match_aria_snapshot() {
    let (_playwright, browser, page) = crate::common::setup().await;

    page.set_content("<h1>Hello</h1><button>Click me</button>", None)
        .await
        .expect("Failed to set content");

    // Page-level assertion (the 1.60 PageAssertions.toMatchAriaSnapshot),
    // matching the whole document rooted at :root.
    expect_page(&page)
        .to_match_aria_snapshot("- heading \"Hello\" [level=1]\n- button \"Click me\"")
        .await
        .expect("page ARIA snapshot should match");

    expect_page(&page)
        .not()
        .with_timeout(std::time::Duration::from_millis(500))
        .to_match_aria_snapshot("- heading \"Goodbye\" [level=1]")
        .await
        .expect("page ARIA snapshot should not match wrong content");

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_aria_snapshot_boxes() {
    let (_playwright, browser, page) = crate::common::setup().await;
    page.set_content("<h1>Hi</h1>", None)
        .await
        .expect("set content");
    let opts = AriaSnapshotOptions::default().boxes(true);
    let snap = page
        .locator("body")
        .aria_snapshot(Some(opts))
        .await
        .expect("aria snapshot with boxes");
    assert!(
        snap.contains("[box="),
        "boxes option should append bounding boxes, got:\n{snap}"
    );
    browser.close().await.ok();
}
