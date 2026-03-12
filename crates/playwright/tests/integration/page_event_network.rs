// Integration tests for Page
//
// These tests verify that we can subscribe page network events pages.

use std::sync::Arc;

use playwright_rs::protocol::{Playwright, Viewport};
use tokio::sync::Mutex;

use crate::test_server::TestServer;

#[tokio::test]
async fn test_page_support_network_events() {
    crate::common::init_tracing();

    let server = TestServer::start().await;

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let events = Arc::new(Mutex::new(vec![]));

    let page = browser.new_page().await.expect("Failed to create page");
    let page_without_events = browser.new_page().await.expect("Failed to create page");

    let events2 = events.clone();
    page.on_request(move |request| {
        let events = events2.clone();
        async move {
            events
                .lock()
                .await
                .push(format!("{} {}", request.method(), request.url()));
            Ok(())
        }
    })
    .await
    .expect("Failed to set request handler");

    let events2 = events.clone();
    page.on_response(move |response| {
        let events = events2.clone();
        async move {
            events
                .lock()
                .await
                .push(format!("{} {}", response.status(), response.url()));
            Ok(())
        }
    })
    .await
    .expect("Failed to set resposne handler");

    let events2 = events.clone();
    page.on_request_finished(move |response| {
        let events = events2.clone();
        async move {
            events.lock().await.push(format!("DONE {}", response.url()));
            Ok(())
        }
    })
    .await
    .expect("Failed to set request finished handler");

    let events2 = events.clone();
    page.on_request_failed(move |response| {
        let events = events2.clone();
        async move {
            events.lock().await.push(format!("FAIL {}", response.url()));
            Ok(())
        }
    })
    .await
    .expect("Failed to set request failed handler");

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    let events: &mut Vec<_> = events.lock().await.as_mut();

    // Since events are dispatched via `tokio::spawn`, we cannot guarantee the order of events.
    // So we sort them before asserting.
    // See more in file `crates/playwright/src/protocol/browser_context.rs`:
    // * BrowserContext::dispatch_request_event
    // * BrowserContext::dispatch_response_event
    events.sort();

    assert_eq!(events.len(), 3);
    assert_eq!(Some(&format!("GET {}/", server.url())), events.get(0));
    assert_eq!(Some(&format!("200 {}/", server.url())), events.get(1));
    assert_eq!(Some(&format!("DONE {}/", server.url())), events.get(2));

    browser.close().await.expect("Failed to close browser");

    server.shutdown();
}

#[tokio::test]
async fn test_for_iframes() {
    crate::common::init_tracing();

    let server = TestServer::start().await;

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let events = Arc::new(Mutex::new(vec![]));

    let page = browser.new_page().await.expect("Failed to create page");

    let events2 = events.clone();
    page.on_request(move |request| {
        let events = events2.clone();
        async move {
            events.lock().await.push(format!("GET {}", request.url()));
            Ok(())
        }
    })
    .await
    .expect("Failed to set request handler");

    let events2 = events.clone();
    page.on_response(move |response| {
        let events = events2.clone();
        async move {
            events
                .lock()
                .await
                .push(format!("{} {}", response.status(), response.url()));
            Ok(())
        }
    })
    .await
    .expect("Failed to set request handler");

    page.goto(&format!("{}/frame.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let events: &mut Vec<_> = events.lock().await.as_mut();

    // Since events are dispatched via `tokio::spawn`, we cannot guarantee the order of events.
    // So we sort them before asserting.
    // See more in file `crates/playwright/src/protocol/browser_context.rs`:
    // * BrowserContext::dispatch_request_event
    // * BrowserContext::dispatch_response_event
    events.sort();

    assert_eq!(4, events.len());
    assert_eq!(
        &vec![
            format!("200 {}/button.html", server.url()),
            format!("200 {}/frame.html", server.url()),
            format!("GET {}/button.html", server.url()),
            format!("GET {}/frame.html", server.url()),
        ],
        events
    );

    browser.close().await.expect("Failed to close browser");

    server.shutdown();
}
