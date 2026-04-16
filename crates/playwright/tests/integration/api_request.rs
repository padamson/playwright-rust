// playwright.request — public APIRequest for headless API testing

use playwright_rs::protocol::Playwright;
use playwright_rs::{APIRequestContextOptions, APIResponse};

use crate::test_server::TestServer;

#[tokio::test]
async fn test_api_request_get() {
    crate::common::init_tracing();
    let server = TestServer::start().await;

    let playwright = Playwright::launch()
        .await
        .expect("setup: failed to launch Playwright");

    let ctx = playwright
        .request()
        .new_context(None)
        .await
        .expect("Failed to create APIRequestContext");

    let url = format!("{}/api/data.json", server.url());
    let response: APIResponse = ctx.get(&url, None).await.expect("GET should succeed");

    assert_eq!(response.status(), 200);
    assert!(response.ok());

    #[derive(serde::Deserialize)]
    struct Data {
        status: String,
        message: String,
    }

    let data: Data = response.json().await.expect("JSON parse should succeed");
    assert_eq!(data.status, "ok");
    assert_eq!(data.message, "hello from test server");

    ctx.dispose().await.expect("dispose should succeed");
    playwright
        .shutdown()
        .await
        .expect("shutdown should succeed");
    server.shutdown();
}

#[tokio::test]
async fn test_api_request_post() {
    crate::common::init_tracing();
    let server = TestServer::start().await;

    let playwright = Playwright::launch()
        .await
        .expect("setup: failed to launch Playwright");

    let ctx = playwright
        .request()
        .new_context(None)
        .await
        .expect("Failed to create APIRequestContext");

    let url = format!("{}/api/echo", server.url());

    use playwright_rs::FetchOptions;
    let opts = FetchOptions {
        method: Some("POST".to_string()),
        post_data: Some("hello post".to_string()),
        ..Default::default()
    };

    let response = ctx
        .post(&url, Some(opts))
        .await
        .expect("POST should succeed");

    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("text() should succeed");
    assert!(body.contains("hello post"));

    ctx.dispose().await.expect("dispose should succeed");
    playwright
        .shutdown()
        .await
        .expect("shutdown should succeed");
    server.shutdown();
}

#[tokio::test]
async fn test_api_request_with_base_url() {
    crate::common::init_tracing();
    let server = TestServer::start().await;

    let playwright = Playwright::launch()
        .await
        .expect("setup: failed to launch Playwright");

    let opts = APIRequestContextOptions {
        base_url: Some(server.url()),
        ..Default::default()
    };

    let ctx = playwright
        .request()
        .new_context(Some(opts))
        .await
        .expect("Failed to create APIRequestContext with base_url");

    let response = ctx
        .get("/api/data.json", None)
        .await
        .expect("GET with relative URL should succeed");

    assert_eq!(response.status(), 200);

    ctx.dispose().await.expect("dispose should succeed");
    playwright
        .shutdown()
        .await
        .expect("shutdown should succeed");
    server.shutdown();
}

#[tokio::test]
async fn test_api_request_dispose() {
    crate::common::init_tracing();

    let playwright = Playwright::launch()
        .await
        .expect("setup: failed to launch Playwright");

    let ctx = playwright
        .request()
        .new_context(None)
        .await
        .expect("Failed to create APIRequestContext");

    ctx.dispose().await.expect("dispose() should succeed");
    playwright
        .shutdown()
        .await
        .expect("shutdown should succeed");
}
