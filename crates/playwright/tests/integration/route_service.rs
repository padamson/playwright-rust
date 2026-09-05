use crate::test_server::TestServer;
use axum::{Json, Router, extract::Path, http::HeaderMap, response::Html, routing::get};
use playwright_rs::expect;
use playwright_rs::protocol::route_service::ServiceRequest;
use tower_http::services::ServeDir;

fn app() -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { Html("<html><head><title>In-process</title></head><body><h1>served by axum</h1></body></html>") }),
        )
        .route(
            "/api/todos",
            get(|| async { Json(serde_json::json!({ "todos": ["write tests"] })) }),
        )
        .route(
            "/api/echo",
            axum::routing::post(|headers: HeaderMap, body: String| async move {
                let probe = headers
                    .get("x-probe")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                format!("{probe}|{body}")
            }),
        )
        .route(
            "/items/{id}",
            get(|Path(id): Path<String>| async move { format!("item {id}") }),
        )
        .route(
            "/login",
            get(|| async {
                ([("set-cookie", "session=abc; Path=/")], "logged in")
            }),
        )
        .route(
            "/api/me",
            get(|headers: HeaderMap| async move {
                headers
                    .get("cookie")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("no cookie")
                    .to_string()
            }),
        )
}

/// A page on `http://app.test/`, served by `app()` in-process.
async fn app_page() -> (
    playwright_rs::protocol::Playwright,
    playwright_rs::protocol::Browser,
    playwright_rs::protocol::Page,
) {
    let (pw, browser, page) = crate::common::setup().await;
    page.route_service("http://app.test/**", app())
        .await
        .expect("register the app");
    page.goto("http://app.test/", None).await.expect("navigate");
    (pw, browser, page)
}

/// Log in through the service, then ask it who the browser is.
async fn assert_cookie_round_trip(page: &playwright_rs::protocol::Page) {
    page.route_service("http://app.test/**", app())
        .await
        .expect("register the app");
    page.goto("http://app.test/", None).await.expect("navigate");
    let me = page
        .evaluate_value("fetch('/login').then(() => fetch('/api/me')).then(r => r.text())")
        .await
        .expect("log in, then ask who I am");
    assert_eq!(me, "session=abc");
}

#[tokio::test]
async fn cookies_the_service_sets_come_back_on_later_requests() {
    let (_pw, browser, page) = crate::common::setup().await;

    assert_cookie_round_trip(&page).await;

    browser.close().await.expect("close browser");
}

#[tokio::test]
#[ignore]
async fn cookies_round_trip_on_firefox() {
    crate::common::init_tracing();
    let playwright = playwright_rs::protocol::Playwright::launch()
        .await
        .expect("launch playwright");
    let browser = playwright.firefox().launch().await.expect("launch firefox");
    let page = browser.new_page().await.expect("new page");

    assert_cookie_round_trip(&page).await;

    browser.close().await.expect("close browser");
}

#[tokio::test]
#[ignore]
async fn cookies_round_trip_on_webkit() {
    crate::common::init_tracing();
    let playwright = playwright_rs::protocol::Playwright::launch()
        .await
        .expect("launch playwright");
    let browser = playwright.webkit().launch().await.expect("launch webkit");
    let page = browser.new_page().await.expect("new page");

    assert_cookie_round_trip(&page).await;

    browser.close().await.expect("close browser");
}

#[tokio::test]
async fn page_route_service_serves_an_axum_app_on_a_made_up_origin() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.route_service("http://app.test/**", app())
        .await
        .expect("register the app");

    let response = page
        .goto("http://app.test/", None)
        .await
        .expect("navigate")
        .expect("a response");
    assert_eq!(response.status(), 200);
    expect(page.locator("h1"))
        .to_have_text("served by axum")
        .await
        .expect("the app's page rendered");

    let todos = page
        .evaluate_value(
            "fetch('/api/todos').then(r => r.json().then(j => `${r.headers.get('content-type')}|${j.todos[0]}`))",
        )
        .await
        .expect("fetch from the app");
    assert_eq!(todos, "application/json|write tests");

    browser.close().await.expect("close browser");
}

#[tokio::test]
async fn route_service_forwards_method_headers_path_and_body() {
    let (_pw, browser, page) = app_page().await;

    let echoed = page
        .evaluate_value(
            r#"fetch('/api/echo', { method: 'POST', headers: { 'x-probe': 'yes' }, body: 'payload' })
                .then(r => r.text())"#,
        )
        .await
        .expect("post to the app");
    assert_eq!(echoed, "yes|payload");

    let item = page
        .evaluate_value("fetch('/items/42').then(r => r.text())")
        .await
        .expect("get a path parameter route");
    assert_eq!(item, "item 42");

    browser.close().await.expect("close browser");
}

#[tokio::test]
async fn route_service_passes_the_service_status_through() {
    let (_pw, browser, page) = app_page().await;

    let status = page
        .evaluate_value("fetch('/missing').then(r => String(r.status))")
        .await
        .expect("fetch an unrouted path");
    assert_eq!(status, "404");

    browser.close().await.expect("close browser");
}

#[tokio::test]
async fn route_service_serves_static_files_with_serve_dir() {
    let (_pw, browser, page) = crate::common::setup().await;
    let dist = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dist.path().join("index.html"),
        "<!doctype html><html><head><title>Static</title><script src=\"app.js\"></script></head><body></body></html>",
    )
    .expect("write index.html");
    std::fs::write(
        dist.path().join("app.js"),
        "document.addEventListener('DOMContentLoaded', () => { document.body.textContent = 'served from disk'; });",
    )
    .expect("write app.js");

    page.route_service("http://static.test/**", ServeDir::new(dist.path()))
        .await
        .expect("register the directory");
    page.goto("http://static.test/", None)
        .await
        .expect("navigate");

    expect(page.locator("body"))
        .to_have_text("served from disk")
        .await
        .expect("the script the directory served ran");
    let title = page
        .evaluate_value("document.title")
        .await
        .expect("read title");
    assert_eq!(title, "Static");

    browser.close().await.expect("close browser");
}

#[tokio::test]
async fn route_service_works_on_an_https_origin_without_a_certificate() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.route_service("https://secure.test/**", app())
        .await
        .expect("register the app");
    page.goto("https://secure.test/", None)
        .await
        .expect("navigate to the https origin");

    let secure = page
        .evaluate_value("String(window.isSecureContext) + ':' + location.protocol")
        .await
        .expect("probe the security context");
    assert_eq!(secure, "true:https:");

    browser.close().await.expect("close browser");
}

#[tokio::test]
async fn context_route_service_applies_to_every_page() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    context
        .route_service("http://app.test/**", app())
        .await
        .expect("register the app on the context");

    for _ in 0..2 {
        let page = context.new_page().await.expect("new page");
        page.goto("http://app.test/", None).await.expect("navigate");
        let title = page
            .evaluate_value("document.title")
            .await
            .expect("read title");
        assert_eq!(title, "In-process");
        page.close().await.expect("close page");
    }

    context.close().await.expect("close context");
    browser.close().await.expect("close browser");
}

#[tokio::test]
async fn a_failing_service_aborts_the_request() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let broken = tower::service_fn(|_request: ServiceRequest| async {
        Err::<http::Response<http_body_util::Full<bytes::Bytes>>, _>(std::io::Error::other("boom"))
    });
    page.route_service("http://broken.test/**", broken)
        .await
        .expect("register the failing service");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("navigate to a real page");
    let outcome = page
        .evaluate_value(
            "fetch('http://broken.test/x').then(() => 'fulfilled').catch(() => 'failed')",
        )
        .await
        .expect("fetch from the failing service");
    assert_eq!(outcome, "failed");

    assert!(
        page.goto("http://broken.test/", None).await.is_err(),
        "a navigation the service fails should error, not hang"
    );

    browser.close().await.expect("close browser");
    server.shutdown();
}

#[tokio::test]
async fn requests_outside_the_pattern_reach_the_network() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.route_service("http://app.test/**", app())
        .await
        .expect("register the app");

    let response = page
        .goto(&format!("{}/api/echo-request", server.url()), None)
        .await
        .expect("navigate to the real server")
        .expect("a response");
    assert_eq!(response.status(), 200);
    assert_eq!(
        crate::common::echoed_request(&page).await["path"],
        "/api/echo-request"
    );

    browser.close().await.expect("close browser");
    server.shutdown();
}

#[tokio::test]
#[ignore]
async fn a_redirecting_service_fails_fast_on_webkit_instead_of_hanging() {
    crate::common::init_tracing();
    let playwright = playwright_rs::protocol::Playwright::launch()
        .await
        .expect("launch playwright");
    let browser = playwright.webkit().launch().await.expect("launch webkit");
    let page = browser.new_page().await.expect("new page");

    let redirecting = tower::service_fn(|_request: ServiceRequest| async {
        Ok::<_, std::convert::Infallible>(
            http::Response::builder()
                .status(302)
                .header("location", "/elsewhere")
                .body(http_body_util::Full::new(bytes::Bytes::new()))
                .unwrap(),
        )
    });
    page.route_service("http://app.test/**", redirecting)
        .await
        .expect("register the redirecting service");

    let started = std::time::Instant::now();
    let outcome = page.goto("http://app.test/", None).await;
    assert!(
        outcome.is_err(),
        "WebKit cannot fulfill a 3xx, so the request must fail"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "the failure must come from the abort, not the navigation timeout"
    );

    browser.close().await.expect("close browser");
}
