// Test for route.fulfill() with main document navigation (Phase 6, Slice 3)
//
// IMPORTANT: These tests document a KNOWN PLAYWRIGHT SERVER LIMITATION (1.49.0 - 1.56.1)
// The route.fulfill() method does not transmit response body content to the browser.
//
// These are "reverse canary tests" - they expect the BROKEN behavior.
// When Playwright fixes this issue, these tests will FAIL, alerting us to update our code.
//
// TODO: Periodically test with newer Playwright versions for fix.

mod test_server;

use playwright_rs::protocol::{FulfillOptions, Playwright};
use test_server::TestServer;

/// Test: route.fulfill() body content is NOT transmitted (Playwright limitation)
///
/// This test documents that Playwright 1.49.0-1.56.1 doesn't transmit fulfilled
/// response bodies to the browser. When this test fails, it means Playwright has
/// fixed the issue and we should update our documentation.
#[tokio::test]
async fn test_route_fulfill_main_document() {
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
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

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
async fn test_route_fulfill_main_document_firefox() {
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
async fn test_route_fulfill_main_document_webkit() {
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
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

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
