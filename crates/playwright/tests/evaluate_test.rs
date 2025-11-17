// Integration tests for Page.evaluate_value() (Phase 5, Slice 4c)
//
// Tests JavaScript evaluation with return values

mod test_server;

use playwright_rs::protocol::Playwright;
use test_server::TestServer;

#[tokio::test]
async fn test_evaluate_arithmetic() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test simple arithmetic
    let result = page
        .evaluate_value("1 + 1")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "2");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_string() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test string
    let result = page
        .evaluate_value("'hello'")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "hello");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_boolean() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test boolean
    let result = page
        .evaluate_value("true")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "true");

    let result = page
        .evaluate_value("false")
        .await
        .expect("Failed to evaluate");
    assert_eq!(result, "false");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_evaluate_fetch_result() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test fetch success/failure
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

    // Should succeed without routing
    assert_eq!(result, "success");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
