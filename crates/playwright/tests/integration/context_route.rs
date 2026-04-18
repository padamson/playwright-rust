// Integration tests for context.route(), context.unroute(), context.unroute_all()

use crate::test_server::TestServer;

#[tokio::test]
async fn test_context_route_abort_blocks_request() {
    let (_pw, browser, context) = crate::common::setup_context().await;
    let server = TestServer::start().await;

    // Set up context-level route to abort image requests
    context
        .route("**/*.png", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up context route");

    let page = context.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Fetch image - should be aborted by context route
    let result = page
        .evaluate_value(
            r#"
        fetch('/image.png')
            .then(() => 'success')
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");

    assert_eq!(
        result, "failed",
        "Image request should be aborted by context route"
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_context_route_applies_to_all_pages() {
    let (_pw, browser, context) = crate::common::setup_context().await;
    let server = TestServer::start().await;

    // Set up context-level route
    context
        .route("**/*.png", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up context route");

    // Create two pages
    let page1 = context.new_page().await.expect("Failed to create page 1");
    let page2 = context.new_page().await.expect("Failed to create page 2");

    page1
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate page 1");
    page2
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate page 2");

    // Both pages should have image requests blocked
    let result1 = page1
        .evaluate_value(
            r#"
        fetch('/image.png')
            .then(() => 'success')
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate on page 1");

    let result2 = page2
        .evaluate_value(
            r#"
        fetch('/image.png')
            .then(() => 'success')
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate on page 2");

    assert_eq!(
        result1, "failed",
        "Page 1 image should be aborted by context route"
    );
    assert_eq!(
        result2, "failed",
        "Page 2 image should be aborted by context route"
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_context_unroute_removes_handler() {
    let (_pw, browser, context) = crate::common::setup_context().await;
    let server = TestServer::start().await;

    // Set up context-level route
    context
        .route("**/*.png", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up context route");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Verify abort works
    let result = page
        .evaluate_value(
            r#"
        fetch('/image.png')
            .then(() => 'success')
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "failed", "Image should be aborted");

    // Unroute
    context
        .unroute("**/*.png")
        .await
        .expect("Failed to unroute");

    // Should no longer be aborted
    let result = page
        .evaluate_value(
            r#"
        fetch('/image.png')
            .then(r => 'status:' + r.status)
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");
    assert!(
        result.to_string().starts_with("status:"),
        "After unroute, fetch should not be aborted (got: {})",
        result
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_context_unroute_all() {
    let (_pw, browser, context) = crate::common::setup_context().await;
    let server = TestServer::start().await;

    // Set up multiple context-level routes
    context
        .route("**/*.png", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up route 1");
    context
        .route("**/*.css", |route| async move { route.abort(None).await })
        .await
        .expect("Failed to set up route 2");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Clear all routes
    context
        .unroute_all(None)
        .await
        .expect("Failed to unroute all");

    // Should no longer be aborted
    let result = page
        .evaluate_value(
            r#"
        fetch('/image.png')
            .then(r => 'status:' + r.status)
            .catch(() => 'failed')
        "#,
        )
        .await
        .expect("Failed to evaluate");
    assert!(
        result.to_string().starts_with("status:"),
        "After unroute_all, fetch should not be aborted (got: {})",
        result
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
