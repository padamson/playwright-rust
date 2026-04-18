// CDPSession and Tracing — Chromium-only (CDP is not available in Firefox/WebKit).

// ============================================================================
// CDPSession Tests
// ============================================================================

#[tokio::test]
async fn test_new_cdp_session_basic() {
    crate::common::init_tracing();

    let (playwright, browser, context) = crate::common::setup_context().await;

    let page = context.new_page().await.expect("Failed to create page");

    // Create a CDP session for the page
    let session = context
        .new_cdp_session(&page)
        .await
        .expect("Failed to create CDP session");

    // Send a simple CDP command: Runtime.evaluate with expression "1+1"
    let result = session
        .send(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "1+1" })),
        )
        .await
        .expect("Failed to send CDP command");

    // The CDP response is double-nested:
    // result = { "result": { "result": { "description": "2", "type": "number", "value": 2 } } }
    // Outer "result" is the protocol envelope field; inner "result" is the CDP returnValue.
    let value = result
        .get("result")
        .and_then(|r| r.get("result"))
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_f64())
        .expect("Expected numeric result from Runtime.evaluate");
    assert_eq!(value as i64, 2, "1+1 should equal 2");

    // Detach the session
    session
        .detach()
        .await
        .expect("Failed to detach CDP session");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    drop(playwright);
}

#[tokio::test]
async fn test_new_cdp_session_send_no_params() {
    crate::common::init_tracing();

    let (playwright, browser, context) = crate::common::setup_context().await;

    let page = context.new_page().await.expect("Failed to create page");

    let session = context
        .new_cdp_session(&page)
        .await
        .expect("Failed to create CDP session");

    // Send a CDP command with no params — Runtime.getIsolateId
    let result = session
        .send("Runtime.getIsolateId", None)
        .await
        .expect("Failed to send CDP command without params");

    // The response is double-nested: { "result": { "id": "..." } }
    // The outer "result" is the Playwright protocol envelope; inner is the CDP response.
    let inner = result
        .get("result")
        .expect("Expected result in CDP response");
    assert!(
        inner.get("id").is_some(),
        "Runtime.getIsolateId should return an id, got: {:?}",
        inner
    );

    session
        .detach()
        .await
        .expect("Failed to detach CDP session");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    drop(playwright);
}

#[tokio::test]
async fn test_new_cdp_session_navigate_and_evaluate() {
    crate::common::init_tracing();

    let (playwright, browser, context) = crate::common::setup_context().await;

    let page = context.new_page().await.expect("Failed to create page");

    // Navigate to a page first
    page.goto(
        "data:text/html,<html><body><h1 id='title'>Hello CDP</h1></body></html>",
        None,
    )
    .await
    .expect("Failed to navigate");

    let session = context
        .new_cdp_session(&page)
        .await
        .expect("Failed to create CDP session");

    // Use CDP to evaluate document.title
    let result = session
        .send(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": "document.getElementById('title').textContent"
            })),
        )
        .await
        .expect("Failed to send CDP command");

    let value = result
        .get("result")
        .and_then(|r| r.get("result"))
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_str())
        .expect("Expected string result");
    assert_eq!(value, "Hello CDP");

    session
        .detach()
        .await
        .expect("Failed to detach CDP session");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    drop(playwright);
}

// ============================================================================
// Tracing Tests
// ============================================================================

#[tokio::test]
async fn test_tracing_start_stop() {
    crate::common::init_tracing();

    let (playwright, browser, context) = crate::common::setup_context().await;

    // Get the tracing object from the context
    let tracing = context
        .tracing()
        .await
        .expect("Failed to get tracing object");

    // Start tracing with no options
    tracing.start(None).await.expect("Failed to start tracing");

    // Do some activity to trace
    let page = context.new_page().await.expect("Failed to create page");
    page.goto(
        "data:text/html,<html><body>Tracing test</body></html>",
        None,
    )
    .await
    .expect("Failed to navigate");

    // Stop tracing — just verify it succeeds without saving to a file
    tracing.stop(None).await.expect("Failed to stop tracing");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    drop(playwright);
}

#[tokio::test]
async fn test_tracing_start_with_options() {
    crate::common::init_tracing();

    let (playwright, browser, context) = crate::common::setup_context().await;

    let tracing = context
        .tracing()
        .await
        .expect("Failed to get tracing object");

    // Start tracing with name and screenshot options
    use playwright_rs::protocol::TracingStartOptions;
    let options = TracingStartOptions {
        name: Some("test-trace".to_string()),
        screenshots: Some(true),
        snapshots: Some(true),
    };

    tracing
        .start(Some(options))
        .await
        .expect("Failed to start tracing with options");

    // Do some activity
    let page = context.new_page().await.expect("Failed to create page");
    page.goto(
        "data:text/html,<html><body>Tracing with options test</body></html>",
        None,
    )
    .await
    .expect("Failed to navigate");

    // Stop tracing
    tracing
        .stop(None)
        .await
        .expect("Failed to stop tracing with options");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    drop(playwright);
}

#[tokio::test]
async fn test_tracing_stop_with_path() {
    crate::common::init_tracing();

    let (playwright, browser, context) = crate::common::setup_context().await;

    let tracing = context
        .tracing()
        .await
        .expect("Failed to get tracing object");

    // Start tracing with snapshots so there's data to save
    use playwright_rs::protocol::TracingStartOptions;
    let start_options = TracingStartOptions {
        name: Some("path-test-trace".to_string()),
        screenshots: Some(false),
        snapshots: Some(true),
    };

    tracing
        .start(Some(start_options))
        .await
        .expect("Failed to start tracing");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(
        "data:text/html,<html><body>Trace to file</body></html>",
        None,
    )
    .await
    .expect("Failed to navigate");

    // Stop tracing and save to a temp file
    let temp_path = std::env::temp_dir().join(format!(
        "pw-rust-trace-test-{}.zip",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    use playwright_rs::protocol::TracingStopOptions;
    let stop_options = TracingStopOptions {
        path: Some(temp_path.to_str().unwrap().to_string()),
    };

    tracing
        .stop(Some(stop_options))
        .await
        .expect("Failed to stop tracing with path");

    // The trace file should exist if the path was provided
    // Note: actual file creation depends on Playwright artifact handling
    // For now just verify the stop succeeded

    // Cleanup
    let _ = std::fs::remove_file(&temp_path);

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    drop(playwright);
}

#[tokio::test]
async fn test_tracing_multiple_start_stop_cycles() {
    crate::common::init_tracing();

    let (playwright, browser, context) = crate::common::setup_context().await;

    let tracing = context
        .tracing()
        .await
        .expect("Failed to get tracing object");

    // First cycle
    tracing
        .start(None)
        .await
        .expect("Failed to start tracing (cycle 1)");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto("data:text/html,<html><body>Cycle 1</body></html>", None)
        .await
        .expect("Failed to navigate (cycle 1)");

    tracing
        .stop(None)
        .await
        .expect("Failed to stop tracing (cycle 1)");

    // Second cycle — should work again
    tracing
        .start(None)
        .await
        .expect("Failed to start tracing (cycle 2)");

    page.goto("data:text/html,<html><body>Cycle 2</body></html>", None)
        .await
        .expect("Failed to navigate (cycle 2)");

    tracing
        .stop(None)
        .await
        .expect("Failed to stop tracing (cycle 2)");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    drop(playwright);
}
