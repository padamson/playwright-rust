// expect_event() — generic event waiting on Page and BrowserContext

use crate::test_server::TestServer;

#[tokio::test]
async fn test_page_expect_event_dispatches_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // "request" event
    let waiter = page.expect_event("request", None).await?;
    page.goto(&server.url(), None).await?;
    let value = waiter.wait().await?;
    assert!(
        matches!(value, playwright_rs::EventValue::Request(_)),
        "Expected EventValue::Request"
    );

    // "console" event
    let waiter = page.expect_event("console", None).await?;
    page.evaluate_expression("console.log('test')").await?;
    let value = waiter.wait().await?;
    assert!(
        matches!(value, playwright_rs::EventValue::ConsoleMessage(_)),
        "Expected EventValue::ConsoleMessage"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

#[tokio::test]
async fn test_page_expect_event_invalid_name() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let result = page.expect_event("nonexistent", None).await;
    assert!(result.is_err(), "Invalid event name should return error");

    browser.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_context_expect_event_dispatches_correctly() -> Result<(), Box<dyn std::error::Error>>
{
    let (_pw, browser, context) = crate::common::setup_context().await;

    // "page" event
    let waiter = context.expect_event("page", None).await?;
    let _page = context.new_page().await?;
    let value = waiter.wait().await?;
    assert!(
        matches!(value, playwright_rs::EventValue::Page(_)),
        "Expected EventValue::Page"
    );

    browser.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_context_expect_event_invalid_name() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let result = context.expect_event("nonexistent", None).await;
    assert!(result.is_err(), "Invalid event name should return error");

    browser.close().await?;
    Ok(())
}
