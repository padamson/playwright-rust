// playwright.request — public APIRequest for headless API testing

use playwright_rs::protocol::Playwright;
use playwright_rs::protocol::{ClearCookiesOptions, Cookie};
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

    // HTTPS should populate both accessors. The test server speaks only plain
    // HTTP, so the populated case needs a real TLS site.
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

async fn cookie_header_sent(
    ctx: &playwright_rs::protocol::APIRequestContext,
    url: &str,
) -> Option<String> {
    let echoed: serde_json::Value = ctx
        .get(url, None)
        .await
        .expect("GET")
        .json()
        .await
        .expect("echo json");
    echoed["headers"]["cookie"].as_str().map(str::to_owned)
}

fn names(cookies: &[Cookie]) -> Vec<&str> {
    cookies.iter().map(|c| c.name.as_str()).collect()
}

#[tokio::test]
async fn request_context_cookies_are_added_read_and_sent() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await.expect("launch");
    let ctx = playwright
        .request()
        .new_context(None)
        .await
        .expect("context");

    ctx.add_cookies(&[
        Cookie::new("a", "b").url(format!("{}/empty.html", server.url())),
        Cookie::new("c", "d")
            .domain("127.0.0.1")
            .path("/api")
            .http_only(true)
            .same_site("Strict"),
        Cookie::new("e", "f").domain("other.com"),
    ])
    .await
    .expect("add_cookies");

    let echo = format!("{}/api/echo-request", server.url());
    assert_eq!(
        cookie_header_sent(&ctx, &echo).await.as_deref(),
        Some("a=b; c=d")
    );

    let all = ctx.cookies(None).await.expect("cookies");
    assert_eq!(names(&all), ["a", "c", "e"]);
    let c = &all[1];
    assert_eq!(
        (c.path.as_str(), c.http_only, c.same_site.as_deref()),
        ("/api", true, Some("Strict"))
    );
    assert_eq!(
        all[0].same_site.as_deref(),
        Some("Lax"),
        "sameSite defaults to Lax"
    );
    assert_eq!(
        names(
            &ctx.cookies(Some(&[&format!("{}/empty.html", server.url())]))
                .await
                .unwrap()
        ),
        ["a"]
    );
    assert_eq!(
        names(&ctx.cookies(Some(&[&echo])).await.unwrap()),
        ["a", "c"]
    );
    assert!(
        ctx.cookies(Some(&["http://nowhere.test/"]))
            .await
            .unwrap()
            .is_empty()
    );

    let err = ctx
        .add_cookies(&[Cookie::new("a", "b")])
        .await
        .expect_err("a cookie needs a url or a domain");
    assert!(
        err.to_string()
            .contains("Cookie should have a url or a domain/path pair"),
        "{err}"
    );

    ctx.clear_cookies(None).await.expect("clear all");
    assert!(ctx.cookies(None).await.unwrap().is_empty());
    assert_eq!(cookie_header_sent(&ctx, &echo).await, None);

    ctx.dispose().await.expect("dispose");
    server.shutdown();
}

#[tokio::test]
async fn request_context_clear_cookies_filters_by_string_and_regex() {
    let playwright = Playwright::launch().await.expect("launch");
    let ctx = playwright
        .request()
        .new_context(None)
        .await
        .expect("context");
    ctx.add_cookies(&[
        Cookie::new("session", "1").domain("one.com"),
        Cookie::new("session", "2").domain("two.com"),
        Cookie::new("session", "3").domain("two.com").path("/api"),
        Cookie::new("other", "4").domain("one.com"),
    ])
    .await
    .expect("add_cookies");
    let values = || async {
        let mut values: Vec<String> = ctx
            .cookies(None)
            .await
            .expect("cookies")
            .into_iter()
            .map(|c| c.value)
            .collect();
        values.sort();
        values
    };

    ctx.clear_cookies(
        ClearCookiesOptions::default()
            .name("session")
            .domain("two.com")
            .path("/api"),
    )
    .await
    .expect("clear by strings");
    assert_eq!(values().await, ["1", "2", "4"]);
    ctx.clear_cookies(ClearCookiesOptions::default().domain_regex(r"one\.com$", ""))
        .await
        .expect("clear by domain regex");
    assert_eq!(values().await, ["2"]);
    ctx.clear_cookies(ClearCookiesOptions::default().name_regex("^SESS", "i"))
        .await
        .expect("clear by name regex with a flag");
    assert!(values().await.is_empty());

    ctx.dispose().await.expect("dispose");
}

#[tokio::test]
async fn a_browser_context_request_shares_the_context_cookie_jar() {
    let server = TestServer::start().await;
    let (_pw, browser, context) = crate::common::setup_context().await;
    let request = context.request().await.expect("context request");
    let url = format!("{}/", server.url());

    request
        .add_cookies(&[Cookie::new("shared", "yes").url(url.clone())])
        .await
        .expect("add through the request context");
    let from_context = context
        .cookies(Some(&[&url]))
        .await
        .expect("context cookies");
    assert_eq!(names(&from_context), ["shared"]);
    assert_eq!(
        names(
            &request
                .cookies(Some(&[&url]))
                .await
                .expect("request cookies")
        ),
        names(&from_context)
    );

    request
        .clear_cookies(None)
        .await
        .expect("clear through the request context");
    assert!(
        context
            .cookies(None)
            .await
            .expect("context cookies")
            .is_empty()
    );

    browser.close().await.expect("close");
    server.shutdown();
}
