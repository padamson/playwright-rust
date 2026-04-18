// Tests for WebError and BrowserContext::on_weberror

use playwright_rs::protocol::EventValue;

#[tokio::test]
async fn test_context_on_weberror() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let page = context.new_page().await.expect("Failed to create page");
    page.goto("about:blank", None)
        .await
        .expect("Failed to navigate");

    // Register the waiter BEFORE the action that triggers the event
    let waiter = context.expect_event("weberror", Some(5000.0)).await?;

    // Throw an uncaught error asynchronously so it escapes the evaluate call
    let _ = page
        .evaluate_expression("setTimeout(() => { throw new Error('test error') }, 0)")
        .await;

    let event = waiter.wait().await.expect("weberror event did not fire");

    let web_error = match event {
        EventValue::WebError(e) => e,
        other => panic!("Expected WebError, got: {:?}", other),
    };

    assert!(
        web_error.error().to_string().contains("test error"),
        "error message should contain 'test error', got: {:?}",
        web_error.error().to_string()
    );
    assert!(
        web_error.page().is_some(),
        "WebError.page() should be Some, but got None"
    );

    browser.close().await.expect("Failed to close browser");
    Ok(())
}
