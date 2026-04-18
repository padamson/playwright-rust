// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Integration tests for Request/Response completion methods:
// - response.security_details(), server_addr(), finished()
// - request.redirected_from / redirected_to
// - request.response()
// - request.sizes()

use crate::test_server::TestServer;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Response server info: security_details, server_addr, finished
// ============================================================================

/// Verify security_details (None for HTTP), server_addr, and finished on a single response.
#[tokio::test]
async fn test_response_server_info() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let response = page
        .goto(&server.url(), None)
        .await?
        .expect("goto should return a response");

    // security_details is None for HTTP
    let details = response.security_details().await?;
    assert!(
        details.is_none(),
        "security_details() should be None for HTTP"
    );

    // server_addr returns localhost
    let addr = response.server_addr().await?;
    assert!(addr.is_some(), "server_addr() should return Some for HTTP");
    let addr = addr.unwrap();
    assert_eq!(addr.ip_address, "127.0.0.1");
    assert!(addr.port > 0);

    // finished resolves for a completed response
    response.finished().await?;

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// request.redirected_from / redirected_to
// ============================================================================

/// Verify redirect chain: request A redirects to request B.
/// B.redirected_from should be A, and A.redirected_to should be B.
#[tokio::test]
async fn test_request_redirect_chain() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let redirect_url = format!("{}/redirect", server.url());
    let response = page
        .goto(&redirect_url, None)
        .await?
        .expect("goto should return a response");

    let final_request = response.request().expect("response should have a request");

    let redirected_from = final_request.redirected_from();
    assert!(
        redirected_from.is_some(),
        "Final request should have redirected_from"
    );

    let original_request = redirected_from.unwrap();
    assert!(
        original_request.url().contains("/redirect"),
        "Original request URL should contain /redirect, got: {}",
        original_request.url()
    );

    let redirected_to = original_request.redirected_to();
    assert!(
        redirected_to.is_some(),
        "Original request should have redirected_to"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// request.response() and request.sizes()
// ============================================================================

/// Verify request.response() and request.sizes() on a finished request.
#[tokio::test]
async fn test_request_response_and_sizes() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // Capture a finished navigation request
    let captured_request = Arc::new(Mutex::new(None));
    let req_clone = captured_request.clone();

    page.on_request_finished(move |request| {
        let capture = req_clone.clone();
        async move {
            if request.is_navigation_request() {
                *capture.lock().unwrap() = Some(request);
            }
            Ok(())
        }
    })
    .await?;

    let _ = page.goto(&server.url(), None).await?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    let request = captured_request
        .lock()
        .unwrap()
        .take()
        .expect("Should have captured a finished navigation request");

    // request.response()
    let response = request.response().await?;
    assert!(response.is_some(), "request.response() should return Some");
    assert_eq!(response.unwrap().status(), 200);

    // request.sizes()
    let sizes = request.sizes().await?;
    assert!(sizes.response_body_size >= 0);
    assert!(sizes.response_headers_size >= 0);
    assert!(sizes.request_headers_size >= 0);

    browser.close().await?;
    server.shutdown();
    Ok(())
}
