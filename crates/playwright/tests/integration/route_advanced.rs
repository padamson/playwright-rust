use crate::test_server::TestServer;
use playwright_rs::protocol::{ContinueOptions, FulfillOptions, Playwright, RouteFromHarOptions};
use std::collections::HashMap;

/// Fulfill the main document and assert the browser rendered it, not the server's page.
async fn assert_fulfilled_document(
    page: &playwright_rs::protocol::Page,
    server: &TestServer,
    status: u16,
    title: &str,
) {
    let html = format!(
        "<!DOCTYPE html><html><head><title>{title}</title></head>\
         <body><p id=\"content\">Fulfillment worked</p></body></html>"
    );
    page.route(&format!("{}/", server.url()), move |route| {
        let html = html.clone();
        async move {
            let options = FulfillOptions::builder()
                .status(status)
                .body_string(html)
                .content_type("text/html")
                .build();
            route.fulfill(Some(options)).await
        }
    })
    .await
    .expect("Failed to set up route");

    let response = page
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");
    assert_eq!(response.status(), status);

    let delivered = page
        .evaluate_value(
            "document.title + '|' + (document.getElementById('content')?.textContent ?? 'missing')",
        )
        .await
        .expect("Failed to read the document");
    assert_eq!(delivered, format!("{title}|Fulfillment worked"));
}

#[tokio::test]
async fn test_route_continue_with_headers() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.route("**/api/echo-request", |route| async move {
        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "test-value".to_string());
        let options = ContinueOptions::builder().headers(headers).build();
        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/api/echo-request", server.url()), None)
        .await
        .expect("Failed to navigate");

    let echoed = crate::common::echoed_request(&page).await;
    assert_eq!(echoed["headers"]["x-custom-header"], "test-value");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_continue_with_method() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.route("**/api/echo-request", |route| async move {
        let options = ContinueOptions::builder()
            .method("POST".to_string())
            .build();
        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/api/echo-request", server.url()), None)
        .await
        .expect("Failed to navigate");

    assert_eq!(crate::common::echoed_request(&page).await["method"], "POST");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_continue_with_post_data() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.route("**/api/echo-request", |route| async move {
        let options = ContinueOptions::builder()
            .method("POST".to_string())
            .post_data("key=value".to_string())
            .build();
        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/api/echo-request", server.url()), None)
        .await
        .expect("Failed to navigate");

    let echoed = crate::common::echoed_request(&page).await;
    assert_eq!(echoed["method"], "POST");
    assert_eq!(echoed["body"], "key=value");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_continue_with_post_data_bytes() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.route("**/api/echo-request", |route| async move {
        let options = ContinueOptions::builder()
            .method("POST".to_string())
            .post_data_bytes(vec![0x01, 0x02, 0xff])
            .build();
        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/api/echo-request", server.url()), None)
        .await
        .expect("Failed to navigate");

    let echoed = crate::common::echoed_request(&page).await;
    assert_eq!(echoed["method"], "POST");
    assert_eq!(echoed["body_base64"], "AQL/");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_continue_with_url() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let target = format!("{}/api/echo-request", server.url());
    page.route("**/original", move |route| {
        let target = target.clone();
        async move {
            let options = ContinueOptions::builder().url(target).build();
            route.continue_(Some(options)).await
        }
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/original", server.url()), None)
        .await
        .expect("Failed to navigate");

    let echoed = crate::common::echoed_request(&page).await;
    assert_eq!(echoed["path"], "/api/echo-request");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_continue_with_combined_overrides() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.route("**/api/echo-request", |route| async move {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());
        headers.insert("X-Test".to_string(), "123".to_string());
        let options = ContinueOptions::builder()
            .headers(headers)
            .method("POST".to_string())
            .post_data("test=data".to_string())
            .build();
        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/api/echo-request", server.url()), None)
        .await
        .expect("Failed to navigate");

    let echoed = crate::common::echoed_request(&page).await;
    assert_eq!(echoed["method"], "POST");
    assert_eq!(echoed["headers"]["x-custom"], "value");
    assert_eq!(echoed["headers"]["x-test"], "123");
    assert_eq!(echoed["body"], "test=data");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_continue_no_overrides() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.route("**/*", |route| async move { route.continue_(None).await })
        .await
        .expect("Failed to set up route");

    let response = page
        .goto(&format!("{}/api/echo-request", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");
    assert_eq!(response.status(), 200);
    assert_eq!(crate::common::echoed_request(&page).await["method"], "GET");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// route.fulfill() with main document navigation
// ============================================================================

#[tokio::test]
async fn test_route_fulfill_main_document() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    assert_fulfilled_document(&page, &server, 200, "Fulfilled Page").await;

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
#[ignore]
async fn test_route_fulfill_main_document_firefox() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");
    let page = browser.new_page().await.expect("Failed to create page");

    assert_fulfilled_document(&page, &server, 200, "Firefox Fulfilled").await;

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
#[ignore]
async fn test_route_fulfill_main_document_webkit() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let page = browser.new_page().await.expect("Failed to create page");

    assert_fulfilled_document(&page, &server, 200, "WebKit Fulfilled").await;

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_fulfill_main_document_with_status() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    assert_fulfilled_document(&page, &server, 404, "Page Not Found").await;

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_fulfill_fetch_json() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.route("**/api/*", |route| async move {
        let options = FulfillOptions::builder()
            .status(200)
            .json(&serde_json::json!({"status": "ok", "mocked": true}))
            .expect("Failed to create JSON response")
            .build();
        route.fulfill(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let fetched = page
        .evaluate_value(
            r#"
        fetch('/api/test')
            .then(r => r.json().then(j => `${r.status}:${r.headers.get('content-type')}:${j.status}:${j.mocked}`))
        "#,
        )
        .await
        .expect("Failed to fetch");
    assert_eq!(fetched, "200:application/json:ok:true");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_page_route_from_har() {
    let (playwright, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

    let har_path = std::env::temp_dir().join("test_route_from_har.har");
    let har_url = format!("{}/api/har-test", server.url());
    let har_content = serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": { "name": "playwright-rust-test", "version": "0.0.0" },
            "entries": [
                {
                    "startedDateTime": "2024-01-01T00:00:00.000Z",
                    "time": 1,
                    "request": {
                        "method": "GET",
                        "url": har_url,
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "queryString": [],
                        "cookies": [],
                        "headersSize": -1,
                        "bodySize": -1
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/1.1",
                        "headers": [
                            { "name": "content-type", "value": "application/json" }
                        ],
                        "cookies": [],
                        "content": {
                            "size": 17,
                            "mimeType": "application/json",
                            "text": "{\"mocked\":true}"
                        },
                        "redirectURL": "",
                        "headersSize": -1,
                        "bodySize": 17
                    },
                    "cache": {},
                    "timings": { "send": 0, "wait": 1, "receive": 0 }
                }
            ]
        }
    });
    std::fs::write(&har_path, har_content.to_string()).expect("Failed to write HAR file");

    let options = RouteFromHarOptions::default()
        .url(format!("{}/api/har-test", server.url()))
        .not_found("abort");

    page.route_from_har(har_path.to_str().unwrap(), Some(options))
        .await
        .expect("route_from_har should succeed");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let har_url = format!("{}/api/har-test", server.url());
    let fetch_result = page
        .evaluate_value(&format!(
            "fetch('{har_url}').then(r => r.json().then(j => `${{r.status}}:${{j.mocked}}`))"
        ))
        .await
        .expect("Failed to evaluate fetch");
    assert_eq!(fetch_result, "200:true");

    std::fs::remove_file(&har_path).ok();
    browser.close().await.expect("Failed to close browser");
    let _ = playwright;
    server.shutdown();
}

#[tokio::test]
async fn test_context_route_from_har() {
    let server = TestServer::start().await;
    let (playwright, browser, context) = crate::common::setup_context().await;

    let har_path = std::env::temp_dir().join("test_context_route_from_har.har");
    let har_url = format!("{}/api/har-test", server.url());
    let har_content = serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": { "name": "playwright-rust-test", "version": "0.0.0" },
            "entries": [
                {
                    "startedDateTime": "2024-01-01T00:00:00.000Z",
                    "time": 1,
                    "request": {
                        "method": "GET",
                        "url": har_url,
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "queryString": [],
                        "cookies": [],
                        "headersSize": -1,
                        "bodySize": -1
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/1.1",
                        "headers": [
                            { "name": "content-type", "value": "application/json" }
                        ],
                        "cookies": [],
                        "content": {
                            "size": 17,
                            "mimeType": "application/json",
                            "text": "{\"mocked\":true}"
                        },
                        "redirectURL": "",
                        "headersSize": -1,
                        "bodySize": 17
                    },
                    "cache": {},
                    "timings": { "send": 0, "wait": 1, "receive": 0 }
                }
            ]
        }
    });
    std::fs::write(&har_path, har_content.to_string()).expect("Failed to write HAR file");

    let options = RouteFromHarOptions::default()
        .url(format!("{}/api/har-test", server.url()))
        .not_found("fallback");

    context
        .route_from_har(har_path.to_str().unwrap(), Some(options))
        .await
        .expect("context.route_from_har should succeed");

    let page = context.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let har_url = format!("{}/api/har-test", server.url());
    let fetch_result = page
        .evaluate_value(&format!(
            "fetch('{har_url}').then(r => r.json().then(j => `${{r.status}}:${{j.mocked}}`))"
        ))
        .await
        .expect("Failed to evaluate fetch");
    assert_eq!(fetch_result, "200:true");

    std::fs::remove_file(&har_path).ok();
    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    let _ = playwright;
    server.shutdown();
}
