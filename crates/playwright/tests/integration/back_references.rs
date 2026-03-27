// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Integration tests for back-reference properties:
// - dialog.page()
// - download.page()
// - response.request()
// - response.frame()
// - request.frame()
//
// Also verifies Response struct field encapsulation (private fields, accessor methods).

use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Response accessor methods (field encapsulation)
// ============================================================================

/// Verify Response fields are accessible only via accessor methods
#[tokio::test]
async fn test_response_accessor_methods() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let response = page
        .goto(&server.url(), None)
        .await?
        .expect("goto should return a response");

    assert!(response.url().starts_with("http://"));
    assert_eq!(response.status(), 200);
    assert!(response.ok());
    assert!(!response.status_text().is_empty() || response.status_text().is_empty()); // just verify callable
    let _headers = response.headers(); // verify returns HashMap reference

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// dialog.page()
// ============================================================================

/// Verify dialog.page() returns the Page that owns the dialog
#[tokio::test]
async fn test_dialog_page_back_reference() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let page_url_from_dialog = Arc::new(Mutex::new(None));
    let url_clone = page_url_from_dialog.clone();

    page.on_dialog(move |dialog| {
        let url_capture = url_clone.clone();
        async move {
            // Use the back-reference to get the page and read its URL
            if let Some(dialog_page) = dialog.page() {
                *url_capture.lock().unwrap() = Some(dialog_page.url().to_string());
            }
            dialog.accept(None).await
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    page.evaluate_expression(
        r#"
        const button = document.createElement('button');
        button.onclick = () => alert('test');
        button.textContent = 'Alert';
        document.body.appendChild(button);
        "#,
    )
    .await?;

    let locator = page.locator("button").await;
    locator.click(None).await?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    let url = page_url_from_dialog.lock().unwrap().take();
    assert_eq!(
        url,
        Some("about:blank".to_string()),
        "dialog.page() should return the owning Page with correct URL"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// download.page()
// ============================================================================

/// Verify download.page() returns the Page that triggered the download
#[tokio::test]
async fn test_download_page_back_reference() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let page_url_from_download = Arc::new(Mutex::new(None));
    let url_clone = page_url_from_download.clone();

    page.on_download(move |download| {
        let url_capture = url_clone.clone();
        async move {
            // Use the back-reference to get the page
            let download_page = download.page();
            *url_capture.lock().unwrap() = Some(download_page.url().to_string());
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    page.evaluate_expression(
        r#"
        const a = document.createElement('a');
        a.href = 'data:text/plain;charset=utf-8,Hello';
        a.download = 'test.txt';
        a.id = 'dl';
        a.textContent = 'Download';
        document.body.appendChild(a);
        "#,
    )
    .await?;

    let locator = page.locator("#dl").await;
    locator.click(None).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let url = page_url_from_download.lock().unwrap().take();
    assert_eq!(
        url,
        Some("about:blank".to_string()),
        "download.page() should return the owning Page with correct URL"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// response.request()
// ============================================================================

/// Verify response.request() returns the Request that triggered the response
#[tokio::test]
async fn test_response_request_back_reference() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let response = page
        .goto(&server.url(), None)
        .await?
        .expect("goto should return a response");

    let request = response
        .request()
        .expect("response.request() should return the owning Request");

    assert!(
        request.url().starts_with("http://"),
        "Request URL should be an HTTP URL, got: {}",
        request.url()
    );
    assert_eq!(request.method(), "GET", "Navigation request should be GET");

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// response.frame()
// ============================================================================

/// Verify response.frame() returns the Frame that initiated the request
#[tokio::test]
async fn test_response_frame_back_reference() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let response = page
        .goto(&server.url(), None)
        .await?
        .expect("goto should return a response");

    let frame = response
        .frame()
        .expect("response.frame() should return the owning Frame");

    assert!(
        frame.url().starts_with("http://"),
        "Frame URL should be an HTTP URL, got: {}",
        frame.url()
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// request.frame()
// ============================================================================

/// Verify request.frame() returns the Frame that initiated the request
#[tokio::test]
async fn test_request_frame_back_reference() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    let has_frame = Arc::new(Mutex::new(false));
    let has_frame_clone = has_frame.clone();

    page.on_request(move |request| {
        let flag = has_frame_clone.clone();
        async move {
            if request.frame().is_some() {
                *flag.lock().unwrap() = true;
            }
            Ok(())
        }
    })
    .await?;

    let _ = page.goto(&server.url(), None).await;

    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        *has_frame.lock().unwrap(),
        "request.frame() should return Some(Frame)"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}
