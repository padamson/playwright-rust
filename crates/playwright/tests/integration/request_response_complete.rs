// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Integration tests for Slice 2: Request/Response completion methods
// - response.security_details()
// - response.server_addr()
// - response.finished()
// - request.redirected_from / redirected_to
// - request.response()
// - request.sizes()

use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// response.security_details()
// ============================================================================

/// For HTTP (non-TLS) connections, security_details() should return None.
#[tokio::test]
async fn test_response_security_details_http() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let response = page
        .goto(&server.url(), None)
        .await?
        .expect("goto should return a response");

    let details = response.security_details().await?;
    assert!(
        details.is_none(),
        "security_details() should be None for HTTP connections"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// response.server_addr()
// ============================================================================

/// server_addr() should return the server's IP and port for HTTP connections.
#[tokio::test]
async fn test_response_server_addr() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let response = page
        .goto(&server.url(), None)
        .await?
        .expect("goto should return a response");

    let addr = response.server_addr().await?;
    assert!(addr.is_some(), "server_addr() should return Some for HTTP");

    let addr = addr.unwrap();
    assert_eq!(
        addr.ip_address, "127.0.0.1",
        "Server should be on localhost"
    );
    assert!(addr.port > 0, "Port should be positive");

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// response.finished()
// ============================================================================

/// finished() should resolve successfully for a completed response.
#[tokio::test]
async fn test_response_finished() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let response = page
        .goto(&server.url(), None)
        .await?
        .expect("goto should return a response");

    // finished() should return Ok(()) for a completed response
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
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Navigate to the redirect endpoint (302 → /)
    let redirect_url = format!("{}/redirect", server.url());
    let response = page
        .goto(&redirect_url, None)
        .await?
        .expect("goto should return a response");

    // The final response's request should have a redirected_from
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

    // The original request's redirected_to should point to the final request
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
// request.response()
// ============================================================================

/// request.response() should return the Response for a completed request.
#[tokio::test]
async fn test_request_response() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Capture a request via on_request
    let captured_request = Arc::new(Mutex::new(None));
    let req_clone = captured_request.clone();

    page.on_request(move |request| {
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
        .expect("Should have captured a navigation request");

    let response = request.response().await?;
    assert!(
        response.is_some(),
        "request.response() should return Some for a completed request"
    );

    let resp = response.unwrap();
    assert_eq!(resp.status(), 200, "Response status should be 200");

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// request.sizes()
// ============================================================================

/// request.sizes() should return size information for a completed request.
#[tokio::test]
async fn test_request_sizes() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Capture a request via on_request_finished (sizes need a finished request)
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

    let sizes = request.sizes().await?;
    assert!(
        sizes.response_body_size >= 0,
        "response_body_size should be non-negative"
    );
    assert!(
        sizes.response_headers_size >= 0,
        "response_headers_size should be non-negative"
    );
    assert!(
        sizes.request_headers_size >= 0,
        "request_headers_size should be non-negative"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}
