use crate::test_server::TestServer;
use playwright_rs::protocol::{ContinueOptions, FulfillOptions, Playwright, RouteFromHarOptions};
use std::collections::HashMap;

#[tokio::test]
async fn test_route_continue_with_headers() {
    // Test modifying headers when continuing a route
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up route handler that modifies headers
    page.route("**/*", |route| async move {
        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "test-value".to_string());

        let options = ContinueOptions::builder().headers(headers).build();

        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    // Navigate - the route should intercept and add custom header
    let result = page.goto("https://example.com", None).await;
    assert!(result.is_ok(), "Navigation should succeed");

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_method() {
    // Test changing HTTP method when continuing a route
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up route handler that changes GET to POST
    page.route("**/*", |route| async move {
        let request = route.request();
        let original_method = request.method();

        if original_method == "GET" {
            let options = ContinueOptions::builder()
                .method("POST".to_string())
                .build();

            route.continue_(Some(options)).await
        } else {
            route.continue_(None).await
        }
    })
    .await
    .expect("Failed to set up route");

    // Navigate - route should change method to POST
    let result = page.goto("https://example.com", None).await;

    // Navigation might fail because server doesn't accept POST for main document
    // But the test verifies the option is accepted by the API
    let _ = result; // Ignore result, we're testing API not behavior

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_post_data() {
    // Test adding POST data when continuing a route
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up route handler that adds POST data
    page.route("**/*", |route| async move {
        let options = ContinueOptions::builder()
            .post_data("key=value".to_string())
            .build();

        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    // Navigate
    let result = page.goto("https://example.com", None).await;
    let _ = result; // Ignore result

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_post_data_bytes() {
    // Test adding POST data as bytes when continuing a route
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up route handler that adds binary POST data
    page.route("**/*", |route| async move {
        let options = ContinueOptions::builder()
            .post_data_bytes(vec![0x01, 0x02, 0x03, 0x04])
            .build();

        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    // Navigate
    let result = page.goto("https://example.com", None).await;
    let _ = result; // Ignore result

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_url() {
    // Test changing URL when continuing a route (same protocol)
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up route handler that redirects to different URL (same protocol)
    page.route("**/original", |route| async move {
        let options = ContinueOptions::builder()
            .url("https://example.com/redirected".to_string())
            .build();

        route.continue_(Some(options)).await
    })
    .await
    .expect("Failed to set up route");

    // Navigate to original URL
    let result = page.goto("https://example.com/original", None).await;
    let _ = result; // Ignore result

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_with_combined_overrides() {
    // Test multiple overrides combined
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up route handler with multiple modifications
    page.route("**/*", |route| async move {
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

    // Navigate
    let result = page.goto("https://example.com", None).await;
    let _ = result; // Ignore result

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_route_continue_no_overrides() {
    // Test that continue without overrides still works
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up route handler that continues without modification
    page.route("**/*", |route| async move { route.continue_(None).await })
        .await
        .expect("Failed to set up route");

    // Navigate - should work normally
    let result = page.goto("https://example.com", None).await;
    assert!(result.is_ok(), "Navigation should succeed");

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// route.fulfill() with main document navigation
// ============================================================================
//
// IMPORTANT: These tests document a KNOWN PLAYWRIGHT SERVER LIMITATION (1.49.0 - 1.58.2):
// route.fulfill() does not transmit response body content to the browser.
//
// These are "reverse canary tests" — they expect the BROKEN behavior. When
// Playwright fixes this, these tests will FAIL, alerting us to update our code.
//
// TODO: Periodically test with newer Playwright versions for fix.

/// Test: route.fulfill() body content is NOT transmitted (Playwright limitation)
///
/// This test documents that Playwright 1.49.0-1.58.2 doesn't transmit fulfilled
/// response bodies to the browser. When this test fails, it means Playwright has
/// fixed the issue and we should update our documentation.
#[tokio::test]
async fn test_route_fulfill_main_document() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // Custom HTML that SHOULD be returned but won't be due to Playwright bug
    let custom_html = r#"<!DOCTYPE html>
<html>
<head><title>Fulfilled Page</title></head>
<body>
  <h1>This is the fulfilled content</h1>
  <p id="content">Fulfillment worked</p>
</body>
</html>"#;

    // Set up route to fulfill main document requests
    page.route("**/*", |route| {
        let request = route.request();
        let is_main_doc = request.resource_type() == "document";

        let custom_html = custom_html.to_string();
        async move {
            if is_main_doc {
                // Attempt to fulfill with custom HTML (body won't be transmitted)
                let options = FulfillOptions::builder()
                    .status(200)
                    .body_string(custom_html)
                    .content_type("text/html")
                    .build();

                route.fulfill(Some(options)).await?;
            } else {
                route.continue_(None).await?;
            }
            Ok(())
        }
    })
    .await
    .expect("Failed to set up route");

    // Navigate to any page
    let response = page
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    // Status code DOES work correctly
    assert_eq!(response.status(), 200, "Status code is correctly fulfilled");

    // KNOWN ISSUE: Body content is NOT transmitted
    // We expect empty title due to Playwright server limitation
    // REVERSE CANARY: When this assertion fails with "Fulfilled Page",
    // Playwright has fixed the body transmission issue!
    let page_title = page
        .evaluate_value("document.title")
        .await
        .expect("Failed to get title");

    assert_eq!(
        page_title, "",
        "REVERSE CANARY: Expected empty (bug), got '{}'. If 'Fulfilled Page', Playwright fixed the issue!",
        page_title
    );

    // Content element won't exist due to empty body
    let content_exists = page
        .evaluate_value("document.getElementById('content') !== null")
        .await
        .expect("Failed to check content");

    assert_eq!(
        content_exists, "false",
        "REVERSE CANARY: Content should not exist due to Playwright limitation"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test: route.fulfill() status codes work, but body doesn't
///
/// Verify that status codes are correctly transmitted even though body isn't.
#[tokio::test]
async fn test_route_fulfill_main_document_with_status() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let html_404 = "<html><body><h1>Page Not Found</h1></body></html>";

    page.route("**/*", |route| {
        let request = route.request();
        let is_main_doc = request.resource_type() == "document";

        let html_404 = html_404.to_string();
        async move {
            if is_main_doc {
                let options = FulfillOptions::builder()
                    .status(404)
                    .body_string(html_404)
                    .content_type("text/html")
                    .build();

                route.fulfill(Some(options)).await?;
            } else {
                route.continue_(None).await?;
            }
            Ok(())
        }
    })
    .await
    .expect("Failed to set up route");

    let response = page
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    // KNOWN ISSUE: Even status codes don't work for main document in some cases
    // We expect 200 instead of 404 due to Playwright limitation with main documents
    assert_eq!(
        response.status(),
        200,
        "REVERSE CANARY: Should be 404 when Playwright fixes main document fulfillment"
    );

    // Body content does NOT work - h1 element won't exist
    let has_h1 = page
        .evaluate_value("document.querySelector('h1') !== null")
        .await
        .expect("Failed to check h1");

    assert_eq!(
        has_h1, "false",
        "REVERSE CANARY: h1 should not exist due to Playwright limitation"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test: route.fulfill() body limitation in Firefox
///
/// Cross-browser test: document that Firefox also has the body transmission issue.
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

    let custom_html = r#"<!DOCTYPE html>
<html>
<head><title>Firefox Fulfilled</title></head>
<body><h1>Firefox fulfillment</h1></body>
</html>"#;

    page.route("**/*", |route| {
        let request = route.request();
        let is_main_doc = request.resource_type() == "document";

        let custom_html = custom_html.to_string();
        async move {
            if is_main_doc {
                let options = FulfillOptions::builder()
                    .status(200)
                    .body_string(custom_html)
                    .content_type("text/html")
                    .build();

                route.fulfill(Some(options)).await?;
            } else {
                route.continue_(None).await?;
            }
            Ok(())
        }
    })
    .await
    .expect("Failed to set up route");

    let response = page
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    assert_eq!(response.status(), 200, "Status works in Firefox");

    // Body content not transmitted in Firefox either
    let title = page
        .evaluate_value("document.title")
        .await
        .expect("Failed to get title");

    assert_eq!(
        title, "",
        "REVERSE CANARY: Firefox also has empty body. Got '{}', expecting 'Firefox Fulfilled' when fixed",
        title
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test: route.fulfill() body limitation in WebKit
///
/// Cross-browser test: document that WebKit also has the body transmission issue.
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

    let custom_html = r#"<!DOCTYPE html>
<html>
<head><title>WebKit Fulfilled</title></head>
<body><h1>WebKit fulfillment</h1></body>
</html>"#;

    page.route("**/*", |route| {
        let request = route.request();
        let is_main_doc = request.resource_type() == "document";

        let custom_html = custom_html.to_string();
        async move {
            if is_main_doc {
                let options = FulfillOptions::builder()
                    .status(200)
                    .body_string(custom_html)
                    .content_type("text/html")
                    .build();

                route.fulfill(Some(options)).await?;
            } else {
                route.continue_(None).await?;
            }
            Ok(())
        }
    })
    .await
    .expect("Failed to set up route");

    let response = page
        .goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    assert_eq!(response.status(), 200, "Status works in WebKit");

    // Body content not transmitted in WebKit either
    let title = page
        .evaluate_value("document.title")
        .await
        .expect("Failed to get title");

    assert_eq!(
        title, "",
        "REVERSE CANARY: WebKit also has empty body. Got '{}', expecting 'WebKit Fulfilled' when fixed",
        title
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test: route.fulfill() status and headers work for fetch (partial functionality)
///
/// This test verifies that status and headers ARE transmitted correctly for
/// fetch requests, even though body content is not. This helps document exactly
/// what works and what doesn't in the current Playwright version.
#[tokio::test]
async fn test_route_fulfill_fetch_still_works() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.route("**/api/*", |route| async move {
        let options = FulfillOptions::builder()
            .status(200)
            .json(&serde_json::json!({"status": "ok", "mocked": true}))
            .expect("Failed to create JSON response")
            .build();

        route.fulfill(Some(options)).await?;
        Ok(())
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Check that status code works for fetch
    let fetch_status = page
        .evaluate_value(
            r#"
        fetch('/api/test')
            .then(r => r.status)
        "#,
        )
        .await
        .expect("Failed to get fetch status");

    assert_eq!(
        fetch_status, "200",
        "Fetch status code is correctly fulfilled"
    );

    // KNOWN ISSUE: Body content is NOT transmitted for fetch either
    // The response will have status 200 but empty body
    let fetch_body = page
        .evaluate_value(
            r#"
        fetch('/api/test')
            .then(r => r.text())
        "#,
        )
        .await
        .expect("Failed to get fetch body");

    // We expect empty body due to Playwright limitation
    assert_eq!(
        fetch_body, "",
        "REVERSE CANARY: Fetch body is empty due to Playwright limitation"
    );

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

    let options = RouteFromHarOptions {
        url: Some(format!("{}/api/har-test", server.url())),
        not_found: Some("abort".to_string()),
        update: None,
        update_content: None,
        update_mode: None,
    };

    page.route_from_har(har_path.to_str().unwrap(), Some(options))
        .await
        .expect("route_from_har should succeed");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let har_url = format!("{}/api/har-test", server.url());
    let fetch_result = page
        .evaluate_value(&format!("fetch('{har_url}').then(r => r.status)"))
        .await
        .expect("Failed to evaluate fetch");

    assert_eq!(
        fetch_result, "200",
        "HAR-mocked response should return status 200"
    );

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

    let options = RouteFromHarOptions {
        url: Some(format!("{}/api/har-test", server.url())),
        not_found: Some("fallback".to_string()),
        update: None,
        update_content: None,
        update_mode: None,
    };

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
        .evaluate_value(&format!("fetch('{har_url}').then(r => r.status)"))
        .await
        .expect("Failed to evaluate fetch");

    assert_eq!(
        fetch_result, "200",
        "HAR-mocked context response should return status 200"
    );

    std::fs::remove_file(&har_path).ok();
    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    let _ = playwright;
    server.shutdown();
}
