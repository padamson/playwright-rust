// Integration tests for route.fetch() via APIRequestContext

use crate::test_server::TestServer;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_route_fetch_basic() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let fetch_status = Arc::new(Mutex::new(0u16));
    let status_clone = fetch_status.clone();

    page.route("**/", move |route| {
        let status = status_clone.clone();
        async move {
            // Fetch the actual response via APIRequestContext
            let response = route.fetch(None).await?;
            *status.lock().unwrap() = response.status();

            // Fulfill with the fetched status
            route
                .fulfill(Some(
                    playwright_rs::FulfillOptions::builder()
                        .status(response.status())
                        .build(),
                ))
                .await
        }
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let status = *fetch_status.lock().unwrap();
    assert_eq!(status, 200, "Fetched response should have status 200");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_fetch_response_methods() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let response_ok = Arc::new(Mutex::new(false));
    let has_body = Arc::new(Mutex::new(false));
    let ok_clone = response_ok.clone();
    let body_clone = has_body.clone();

    page.route("**/", move |route| {
        let ok = ok_clone.clone();
        let body = body_clone.clone();
        async move {
            let response = route.fetch(None).await?;

            // Test FetchResponse methods
            *ok.lock().unwrap() = response.ok();
            *body.lock().unwrap() = !response.body().is_empty();

            assert!(response.ok(), "200 response should be ok()");
            assert!(!response.headers().is_empty(), "Should have headers");

            route
                .fulfill(Some(
                    playwright_rs::FulfillOptions::builder()
                        .status(response.status())
                        .build(),
                ))
                .await
        }
    })
    .await
    .expect("Failed to set up route");

    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    assert!(*response_ok.lock().unwrap(), "Response should have been ok");
    assert!(*has_body.lock().unwrap(), "Response should have a body");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_route_fetch_with_context_route() {
    let server = TestServer::start().await;
    let (_pw, browser, context) = crate::common::setup_context().await;

    let fetch_status = Arc::new(Mutex::new(0u16));
    let status_clone = fetch_status.clone();

    // Use context-level route with fetch
    context
        .route("**/", move |route| {
            let status = status_clone.clone();
            async move {
                let response = route.fetch(None).await?;
                *status.lock().unwrap() = response.status();
                route
                    .fulfill(Some(
                        playwright_rs::FulfillOptions::builder()
                            .status(response.status())
                            .build(),
                    ))
                    .await
            }
        })
        .await
        .expect("Failed to set up context route");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate");

    let status = *fetch_status.lock().unwrap();
    assert_eq!(
        status, 200,
        "Context-level route.fetch() should get status 200"
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
