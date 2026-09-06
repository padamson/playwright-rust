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
    let opts = FetchOptions::builder()
        .method("POST".to_string())
        .post_data("hello post".to_string())
        .build();

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

    let opts = APIRequestContextOptions::default().base_url(server.url());

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

#[tokio::test]
async fn test_api_response_server_addr_and_security_details() {
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

    // Resource timing arrives in the response initializer (Playwright 1.62),
    // so unlike the browser-side Request::timing it needs no event to have
    // fired first. Phases that were never reached read as -1, so assert on
    // start_time, which is always populated.
    let timing = response
        .timing()
        .expect("a completed fetch should report timing");
    assert!(
        timing.start_time > 0.0,
        "start_time should be a real epoch millisecond value, got {}",
        timing.start_time
    );
    assert!(
        timing.request_start >= 0.0,
        "request_start should have been reached, got {}",
        timing.request_start
    );

    // The driver builds the response before the body finishes, so it reports
    // the end separately as `responseEndTiming`. We fold that back in, the
    // same way the browser-side Request::timing path does, so `response_end`
    // means the same thing whichever origin produced the ResourceTiming.
    let end = response
        .response_end_timing()
        .expect("responseEndTiming should be reported for a completed fetch");
    assert_eq!(
        timing.response_end, end,
        "response_end should be folded in from responseEndTiming, not left at -1"
    );
    assert!(
        timing.response_end >= timing.request_start,
        "response_end ({}) should not precede request_start ({})",
        timing.response_end,
        timing.request_start
    );

    // Plain HTTP carries no TLS details, and the server omits a remote address
    // for this fetch. The accessors must resolve cleanly (the new initializer
    // fields must not break fetch deserialization).
    assert!(
        response.security_details().is_none(),
        "plain HTTP should have no security details"
    );
    if let Some(addr) = response.server_addr() {
        assert!(addr.port > 0, "if present, server port should be set");
    }

    // HTTPS should populate both accessors (the populated case, matching the
    // `page.goto("https://example.com", ...)` precedent used elsewhere in
    // this suite for real-network TLS coverage).
    let https_response: APIResponse = ctx
        .get("https://example.com", None)
        .await
        .expect("HTTPS GET should succeed");
    let security_details = https_response
        .security_details()
        .expect("HTTPS response should carry security details");
    assert!(
        security_details.protocol.as_deref().unwrap_or_default() != "",
        "security details should report a TLS protocol version"
    );
    let addr = https_response
        .server_addr()
        .expect("HTTPS response should carry a server address");
    assert_eq!(addr.port, 443, "HTTPS server address should be port 443");

    ctx.dispose().await.expect("dispose should succeed");
    playwright
        .shutdown()
        .await
        .expect("shutdown should succeed");
    server.shutdown();
}

/// An API request context authenticates too, and it is the one place
/// `send(Always)` is honored: the header goes out without a challenge.
#[tokio::test]
async fn api_request_context_sends_credentials_up_front() -> Result<(), Box<dyn std::error::Error>>
{
    use playwright_rs::protocol::{APIRequestContextOptions, HttpCredentials, HttpCredentialsSend};

    let server = crate::test_server::TestServer::start().await;
    let (playwright, browser, _) = crate::common::setup().await;

    // This endpoint answers 403 with no challenge, so waiting to be
    // challenged never authenticates: only `Always` gets through.
    let url = format!("{}/protected-403", server.url());

    let reactive = playwright
        .request()
        .new_context(Some(
            APIRequestContextOptions::default()
                .http_credentials(vec![HttpCredentials::new("user", "secret")]),
        ))
        .await?;
    assert_eq!(reactive.get(&url, None).await?.status(), 403);
    reactive.dispose().await?;

    let up_front = playwright
        .request()
        .new_context(Some(APIRequestContextOptions::default().http_credentials(
            vec![HttpCredentials::new("user", "secret").send(HttpCredentialsSend::Always)],
        )))
        .await?;
    let response = up_front.get(&url, None).await?;
    assert_eq!(response.status(), 200);
    assert!(response.text().await?.contains("secret area"));
    up_front.dispose().await?;
    browser.close().await?;
    server.shutdown();
    Ok(())
}
