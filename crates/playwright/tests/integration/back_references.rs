// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Covers back-reference properties: dialog.page(), download.page(),
// response.request(), response.frame(), request.frame().
// Also verifies Response struct field encapsulation.

use crate::test_server::TestServer;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Response accessors, request(), and frame() — single browser session
// ============================================================================

/// Verify Response field encapsulation plus response.request() and response.frame()
#[tokio::test]
async fn test_response_accessors_and_back_references() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

    let response = page
        .goto(&server.url(), None)
        .await?
        .expect("goto should return a response");

    // Field encapsulation: all accessors work
    assert!(response.url().starts_with("http://"));
    assert_eq!(response.status(), 200);
    assert!(response.ok());
    let _status_text = response.status_text();
    let _headers = response.headers();

    // response.request() back-reference
    let request = response
        .request()
        .expect("response.request() should return the owning Request");
    assert!(request.url().starts_with("http://"));
    assert_eq!(request.method(), "GET");

    // response.frame() back-reference
    let frame = response
        .frame()
        .expect("response.frame() should return the owning Frame");
    assert!(frame.url().starts_with("http://"));

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
    let (_pw, browser, page) = crate::common::setup().await;

    let page_url_from_dialog = Arc::new(Mutex::new(None));
    let url_clone = page_url_from_dialog.clone();

    page.on_dialog(move |dialog| {
        let url_capture = url_clone.clone();
        async move {
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
    assert_eq!(url, Some("about:blank".to_string()));

    browser.close().await?;
    Ok(())
}

// ============================================================================
// download.page()
// ============================================================================

/// Verify download.page() returns the Page that triggered the download
#[tokio::test]
async fn test_download_page_back_reference() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let page_url_from_download = Arc::new(Mutex::new(None));
    let url_clone = page_url_from_download.clone();

    page.on_download(move |download| {
        let url_capture = url_clone.clone();
        async move {
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
    assert_eq!(url, Some("about:blank".to_string()));

    browser.close().await?;
    Ok(())
}

// ============================================================================
// request.frame()
// ============================================================================

/// Verify request.frame() returns the Frame that initiated the request
#[tokio::test]
async fn test_request_frame_back_reference() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

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
