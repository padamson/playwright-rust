// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Integration tests for Page on_* event handlers:
//   on_close, on_load, on_crash, on_pageerror, on_popup,
//   on_frameattached, on_framedetached, on_framenavigated
//
// See: <https://playwright.dev/docs/api/class-page>

use crate::test_server::TestServer;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// on_close
// ============================================================================

/// Test that on_close fires when close() is called.
#[tokio::test]
async fn test_page_on_close() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let fired = Arc::new(Mutex::new(false));
    let fired_clone = fired.clone();

    page.on_close(move || {
        let f = fired_clone.clone();
        async move {
            *f.lock().unwrap() = true;
            Ok(())
        }
    })
    .await?;

    page.close().await?;

    // Give the event loop time to dispatch the close event
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        *fired.lock().unwrap(),
        "on_close handler should have fired after page.close()"
    );

    browser.close().await?;
    Ok(())
}

/// Test that multiple on_close handlers all fire.
#[tokio::test]
async fn test_page_on_close_multiple_handlers() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let count = Arc::new(Mutex::new(0u32));

    for _ in 0..3 {
        let c = count.clone();
        page.on_close(move || {
            let cc = c.clone();
            async move {
                *cc.lock().unwrap() += 1;
                Ok(())
            }
        })
        .await?;
    }

    page.close().await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(
        *count.lock().unwrap(),
        3,
        "All three on_close handlers should have fired"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// on_load
// ============================================================================

/// Test that on_load fires when a page fully loads.
#[tokio::test]
async fn test_page_on_load() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let fired = Arc::new(Mutex::new(false));
    let fired_clone = fired.clone();

    page.on_load(move || {
        let f = fired_clone.clone();
        async move {
            *f.lock().unwrap() = true;
            Ok(())
        }
    })
    .await?;

    page.goto(&server.url(), None).await?;

    // Give the event loop time to dispatch the load event
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        *fired.lock().unwrap(),
        "on_load handler should have fired after navigation"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Test that on_load fires for multiple navigations.
#[tokio::test]
async fn test_page_on_load_multiple_navigations() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let count = Arc::new(Mutex::new(0u32));
    let count_clone = count.clone();

    page.on_load(move || {
        let c = count_clone.clone();
        async move {
            *c.lock().unwrap() += 1;
            Ok(())
        }
    })
    .await?;

    page.goto(&server.url(), None).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    page.goto(&format!("{}/button.html", server.url()), None)
        .await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let n = *count.lock().unwrap();
    assert!(n >= 2, "on_load should fire for each navigation, got {}", n);

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// on_pageerror
// ============================================================================

/// Test that on_pageerror fires when JS throws an uncaught exception.
#[tokio::test]
async fn test_page_on_pageerror() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let message = Arc::new(Mutex::new(None::<String>));
    let msg_clone = message.clone();

    page.on_pageerror(move |msg| {
        let m = msg_clone.clone();
        async move {
            *m.lock().unwrap() = Some(msg);
            Ok(())
        }
    })
    .await?;

    // navigate first so there's a real document context
    let _ = page.goto("about:blank", None).await;

    // Throw an uncaught error from a script that runs outside evaluate's promise chain
    page.evaluate::<(), ()>(
        "() => { setTimeout(() => { throw new Error('test-pageerror'); }, 0); }",
        None,
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let result = message.lock().unwrap().clone();
    assert!(
        result.is_some(),
        "on_pageerror handler should have fired for uncaught JS exception"
    );
    let msg = result.unwrap();
    assert!(
        msg.contains("test-pageerror"),
        "Error message should contain 'test-pageerror', got: {msg}"
    );

    browser.close().await?;
    Ok(())
}

/// Test that on_pageerror receives the error message as a string.
#[tokio::test]
async fn test_page_on_pageerror_message_is_string() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let message = Arc::new(Mutex::new(None::<String>));
    let msg_clone = message.clone();

    page.on_pageerror(move |msg| {
        let m = msg_clone.clone();
        async move {
            *m.lock().unwrap() = Some(msg);
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    page.evaluate::<(), ()>(
        "() => { setTimeout(() => { throw new TypeError('type-error-test'); }, 0); }",
        None,
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let result = message.lock().unwrap().clone();
    assert!(result.is_some(), "on_pageerror should have fired");
    let msg = result.unwrap();
    // The handler receives a plain String, not wrapped in any type
    assert!(
        !msg.is_empty(),
        "Error message should be non-empty, got: {msg}"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// on_popup
// ============================================================================

/// Test that on_popup fires when window.open() creates a popup.
#[tokio::test]
async fn test_page_on_popup() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, _browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await?;

    let popup_received = Arc::new(Mutex::new(false));
    let popup_url = Arc::new(Mutex::new(None::<String>));
    let pr_clone = popup_received.clone();
    let pu_clone = popup_url.clone();

    page.on_popup(move |popup_page| {
        let pr = pr_clone.clone();
        let pu = pu_clone.clone();
        async move {
            *pr.lock().unwrap() = true;
            *pu.lock().unwrap() = Some(popup_page.url());
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    // Open a popup
    page.evaluate::<(), ()>("() => { window.open('about:blank'); }", None)
        .await?;

    tokio::time::sleep(Duration::from_millis(800)).await;

    assert!(
        *popup_received.lock().unwrap(),
        "on_popup handler should have fired when window.open() was called"
    );

    context.close().await?;
    Ok(())
}

/// Test that on_popup receives a valid Page object.
#[tokio::test]
async fn test_page_on_popup_receives_page() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, _browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await?;

    let popup_guid = Arc::new(Mutex::new(None::<String>));
    let pg_clone = popup_guid.clone();

    use playwright_rs::server::channel_owner::ChannelOwner;
    page.on_popup(move |popup_page| {
        let pg = pg_clone.clone();
        async move {
            *pg.lock().unwrap() = Some(popup_page.guid().to_string());
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;
    page.evaluate::<(), ()>("() => { window.open('about:blank'); }", None)
        .await?;

    tokio::time::sleep(Duration::from_millis(800)).await;

    let guid = popup_guid.lock().unwrap().clone();
    assert!(guid.is_some(), "Popup page guid should be set");
    let guid = guid.unwrap();
    assert!(
        !guid.is_empty(),
        "Popup page guid should be non-empty, got: {guid}"
    );

    context.close().await?;
    Ok(())
}

// ============================================================================
// on_frameattached / on_framedetached / on_framenavigated
// ============================================================================

/// Test that on_frameattached fires when an iframe is added to the page.
#[tokio::test]
async fn test_page_on_frameattached() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let fired = Arc::new(Mutex::new(false));
    let fired_clone = fired.clone();

    page.on_frameattached(move |_frame| {
        let f = fired_clone.clone();
        async move {
            *f.lock().unwrap() = true;
            Ok(())
        }
    })
    .await?;

    // Navigate to a page with an iframe — this attaches a frame
    page.goto(&format!("{}/frame.html", server.url()), None)
        .await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        *fired.lock().unwrap(),
        "on_frameattached should fire when an iframe is loaded"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Test that on_framedetached fires when an iframe is removed from the page.
#[tokio::test]
async fn test_page_on_framedetached() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let fired = Arc::new(Mutex::new(false));
    let fired_clone = fired.clone();

    page.on_framedetached(move |_frame| {
        let f = fired_clone.clone();
        async move {
            *f.lock().unwrap() = true;
            Ok(())
        }
    })
    .await?;

    // Load a page with a frame, then navigate away to detach it
    page.goto(&format!("{}/frame.html", server.url()), None)
        .await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Navigate away — this detaches the iframe
    page.goto(&server.url(), None).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        *fired.lock().unwrap(),
        "on_framedetached should fire when navigating away from a page with iframes"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Test that on_framenavigated fires when a frame navigates.
#[tokio::test]
async fn test_page_on_framenavigated() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let count = Arc::new(Mutex::new(0u32));
    let count_clone = count.clone();

    page.on_framenavigated(move |_frame| {
        let c = count_clone.clone();
        async move {
            *c.lock().unwrap() += 1;
            Ok(())
        }
    })
    .await?;

    // Navigate the page — the main frame navigates
    page.goto(&server.url(), None).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let n = *count.lock().unwrap();
    assert!(
        n >= 1,
        "on_framenavigated should fire at least once on navigation, got {n}"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// on_crash (marked ignore — page crashes are hard to trigger reliably)
// ============================================================================

/// Test that on_crash can be registered without error.
/// Actual crash testing is skipped — crashes are not reliably triggerable
/// in headless Chromium without OS-level manipulation.
#[tokio::test]
async fn test_page_on_crash_can_register() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, _browser, page) = crate::common::setup().await;

    // Just verify registration doesn't error
    page.on_crash(|| async move { Ok(()) }).await?;

    Ok(())
}

// ============================================================================
// expect_popup
// ============================================================================

/// Test that expect_popup resolves when window.open() creates a popup.
#[tokio::test]
async fn test_page_expect_popup() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, _browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await?;

    let _ = page.goto("about:blank", None).await;

    // Set up waiter BEFORE triggering the popup
    let waiter = page.expect_popup(Some(5000.0)).await?;

    // Trigger popup via window.open
    page.evaluate::<(), ()>("() => { window.open('about:blank'); }", None)
        .await?;

    // Resolve the waiter
    let popup = waiter.wait().await?;

    // Verify we received a valid Page object
    use playwright_rs::server::channel_owner::ChannelOwner;
    assert!(
        !popup.guid().is_empty(),
        "Popup page should have a non-empty GUID"
    );

    context.close().await?;
    Ok(())
}

/// Test that expect_popup times out when no popup opens.
#[tokio::test]
async fn test_page_expect_popup_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up waiter with a very short timeout
    let waiter = page.expect_popup(Some(100.0)).await?;

    // Do NOT trigger any popup — waiter should timeout
    let result = waiter.wait().await;
    assert!(
        result.is_err(),
        "expect_popup should time out when no popup opens"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// expect_download
// ============================================================================

/// Test that expect_download resolves when a download starts.
#[tokio::test]
async fn test_page_expect_download() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let _ = page.goto("about:blank", None).await;

    // Inject a download link
    page.evaluate_expression(
        r#"
        const a = document.createElement('a');
        a.href = 'data:text/plain;charset=utf-8,Hello%20World';
        a.download = 'expect_download_test.txt';
        a.id = 'dl';
        a.textContent = 'Download';
        document.body.appendChild(a);
        "#,
    )
    .await?;

    // Set up waiter BEFORE clicking the download link
    let waiter = page.expect_download(Some(5000.0)).await?;

    // Trigger the download
    page.locator("#dl").await.click(None).await?;

    // Resolve the waiter
    let download = waiter.wait().await?;

    // Verify we received a valid Download object
    assert!(
        download.url().contains("data:text/plain"),
        "Download URL should be a data URL, got: {}",
        download.url()
    );
    assert_eq!(
        download.suggested_filename(),
        "expect_download_test.txt",
        "Suggested filename should match"
    );

    browser.close().await?;
    Ok(())
}

/// Test that expect_download times out when no download starts.
#[tokio::test]
async fn test_page_expect_download_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up waiter with a very short timeout
    let waiter = page.expect_download(Some(100.0)).await?;

    // Do NOT trigger any download
    let result = waiter.wait().await;
    assert!(
        result.is_err(),
        "expect_download should time out when no download occurs"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// expect_response
// ============================================================================

/// Test that expect_response resolves when a network response is received.
#[tokio::test]
async fn test_page_expect_response() -> Result<(), Box<dyn std::error::Error>> {
    let server = crate::test_server::TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up waiter BEFORE navigation (which triggers responses)
    let waiter = page.expect_response(Some(5000.0)).await?;

    // Navigate — this causes at least one HTTP response
    page.goto(&server.url(), None).await?;

    // Resolve the waiter
    let response = waiter.wait().await?;

    // Verify we received a valid ResponseObject
    let status = response.status();
    assert!(
        (100..600).contains(&status),
        "Response status should be a valid HTTP status code, got: {}",
        status
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Test that expect_response times out when no response arrives.
#[tokio::test]
async fn test_page_expect_response_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up waiter with a very short timeout — do not navigate
    let waiter = page.expect_response(Some(100.0)).await?;

    let result = waiter.wait().await;
    assert!(
        result.is_err(),
        "expect_response should time out when no response arrives"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// expect_request
// ============================================================================

/// Test that expect_request resolves when a network request is issued.
#[tokio::test]
async fn test_page_expect_request() -> Result<(), Box<dyn std::error::Error>> {
    let server = crate::test_server::TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up waiter BEFORE navigation (which issues requests)
    let waiter = page.expect_request(Some(5000.0)).await?;

    // Navigate — this issues at least one HTTP request
    page.goto(&server.url(), None).await?;

    // Resolve the waiter
    let request = waiter.wait().await?;

    // Verify we received a valid Request object
    assert!(!request.url().is_empty(), "Request URL should not be empty");
    assert!(
        !request.method().is_empty(),
        "Request method should not be empty"
    );

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Test that expect_request times out when no request is issued.
#[tokio::test]
async fn test_page_expect_request_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up waiter with very short timeout — do not navigate
    let waiter = page.expect_request(Some(100.0)).await?;

    let result = waiter.wait().await;
    assert!(
        result.is_err(),
        "expect_request should time out when no request is issued"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// expect_console_message
// ============================================================================

/// Test that expect_console_message resolves when console.log is called.
#[tokio::test]
async fn test_page_expect_console_message() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let _ = page.goto("about:blank", None).await;

    // Set up waiter BEFORE triggering the console message
    let waiter = page.expect_console_message(Some(5000.0)).await?;

    // Trigger a console.log
    page.evaluate::<(), ()>(
        "() => { console.log('expect_console_message test'); }",
        None,
    )
    .await?;

    // Resolve the waiter
    let msg = waiter.wait().await?;

    // Verify we received a valid ConsoleMessage
    assert_eq!(msg.type_(), "log", "Console message type should be 'log'");
    assert!(
        msg.text().contains("expect_console_message test"),
        "Console message text should contain the logged string, got: {}",
        msg.text()
    );

    browser.close().await?;
    Ok(())
}

/// Test that expect_console_message works for console.error.
#[tokio::test]
async fn test_page_expect_console_message_error_type() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let _ = page.goto("about:blank", None).await;

    let waiter = page.expect_console_message(Some(5000.0)).await?;

    page.evaluate::<(), ()>("() => { console.error('error-message-test'); }", None)
        .await?;

    let msg = waiter.wait().await?;
    assert_eq!(
        msg.type_(),
        "error",
        "Console message type should be 'error'"
    );
    assert!(
        msg.text().contains("error-message-test"),
        "Console message text should match, got: {}",
        msg.text()
    );

    browser.close().await?;
    Ok(())
}

/// Test that expect_console_message times out when no console message is produced.
#[tokio::test]
async fn test_page_expect_console_message_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    // Set up waiter with very short timeout — do not call console.log
    let waiter = page.expect_console_message(Some(100.0)).await?;

    let result = waiter.wait().await;
    assert!(
        result.is_err(),
        "expect_console_message should time out when no console message is produced"
    );

    browser.close().await?;
    Ok(())
}

/// Test that popup.opener() returns the page that opened it.
#[tokio::test]
async fn test_page_opener() -> Result<(), Box<dyn std::error::Error>> {
    use playwright_rs::server::channel_owner::ChannelOwner;

    let (_pw, _browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await?;

    // Set up waiter for popup
    let popup_waiter = page.expect_popup(None).await?;

    // Open a popup via window.open
    page.evaluate::<(), ()>("() => { window.open('about:blank'); }", None)
        .await?;

    let popup = popup_waiter.wait().await?;

    // The popup's opener should be the original page
    let opener = popup.opener().await?;
    assert!(opener.is_some(), "popup.opener() should return Some(page)");
    let opener_page = opener.unwrap();
    assert_eq!(
        opener_page.guid(),
        page.guid(),
        "opener should be the page that opened the popup"
    );

    // A non-popup page should return None
    let non_popup_opener = page.opener().await?;
    assert!(
        non_popup_opener.is_none(),
        "non-popup page.opener() should return None"
    );

    context.close().await?;
    Ok(())
}
