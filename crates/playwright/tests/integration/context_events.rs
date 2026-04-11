// Integration tests for BrowserContext-level event handlers
//
// These tests verify that context-level event handlers fire for events
// across all pages in the context.

use std::sync::{Arc, Mutex};

use playwright_rs::protocol::Playwright;

use crate::test_server::TestServer;

// ---------------------------------------------------------------------------
// on_page
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_context_on_page() {
    crate::common::init_tracing();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    let fired_pages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let fired_pages2 = fired_pages.clone();

    context
        .on_page(move |page| {
            let fired = fired_pages2.clone();
            async move {
                fired.lock().unwrap().push(page.url());
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_page handler");

    // Creating a new page should trigger the handler
    let _page = context.new_page().await.expect("Failed to create page");

    // Give the async event a moment to fire
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    {
        let pages = fired_pages.lock().unwrap();
        assert_eq!(
            pages.len(),
            1,
            "on_page handler should fire once, got: {:?}",
            pages
        );
    }

    browser.close().await.expect("Failed to close browser");
}

// ---------------------------------------------------------------------------
// on_close
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_context_on_close() {
    crate::common::init_tracing();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    let closed = Arc::new(Mutex::new(false));
    let closed2 = closed.clone();

    context
        .on_close(move || {
            let closed = closed2.clone();
            async move {
                *closed.lock().unwrap() = true;
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_close handler");

    context.close().await.expect("Failed to close context");

    // Give the async event a moment to fire
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert!(
        *closed.lock().unwrap(),
        "on_close handler should have fired"
    );

    browser.close().await.expect("Failed to close browser");
}

// ---------------------------------------------------------------------------
// on_request
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_context_on_request() {
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

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    let requests: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let requests2 = requests.clone();

    context
        .on_request(move |request| {
            let reqs = requests2.clone();
            async move {
                reqs.lock()
                    .unwrap()
                    .push(format!("{} {}", request.method(), request.url()));
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_request handler");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    // Give the async event a moment to fire
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    {
        let reqs = requests.lock().unwrap();
        assert!(
            !reqs.is_empty(),
            "on_request handler should have fired, got: {:?}",
            reqs
        );
        let has_get = reqs
            .iter()
            .any(|r| r.contains("GET") && r.contains(&server.url()));
        assert!(has_get, "Expected a GET request in: {:?}", reqs);
    }

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ---------------------------------------------------------------------------
// on_response
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_context_on_response() {
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

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    let responses: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let responses2 = responses.clone();

    context
        .on_response(move |response| {
            let resps = responses2.clone();
            async move {
                resps
                    .lock()
                    .unwrap()
                    .push(format!("{} {}", response.status(), response.url()));
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_response handler");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    // Give the async event a moment to fire
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    {
        let resps = responses.lock().unwrap();
        assert!(
            !resps.is_empty(),
            "on_response handler should have fired, got: {:?}",
            resps
        );
        let has_200 = resps
            .iter()
            .any(|r| r.contains("200") && r.contains(&server.url()));
        assert!(has_200, "Expected a 200 response in: {:?}", resps);
    }

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ---------------------------------------------------------------------------
// on_request_finished
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_context_on_request_finished() {
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

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    let finished: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let finished2 = finished.clone();

    context
        .on_request_finished(move |request| {
            let fin = finished2.clone();
            async move {
                fin.lock().unwrap().push(format!("DONE {}", request.url()));
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_request_finished handler");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    // Give the async event a moment to fire
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    {
        let fin = finished.lock().unwrap();
        assert!(
            !fin.is_empty(),
            "on_request_finished handler should have fired, got: {:?}",
            fin
        );
        let has_done = fin
            .iter()
            .any(|r| r.starts_with("DONE") && r.contains(&server.url()));
        assert!(has_done, "Expected a DONE entry in: {:?}", fin);
    }

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ---------------------------------------------------------------------------
// on_request_failed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_context_on_request_failed() {
    crate::common::init_tracing();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    let failed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let failed2 = failed.clone();

    context
        .on_request_failed(move |request| {
            let f = failed2.clone();
            async move {
                f.lock()
                    .unwrap()
                    .push(format!("FAIL {} {}", request.method(), request.url()));
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_request_failed handler");

    let page = context.new_page().await.expect("Failed to create page");

    // Navigate to a non-routable address to trigger a request failure
    let result = page.goto("http://localhost:1", None).await;
    assert!(result.is_err(), "Navigation to bad port should fail");

    // Give the async event a moment to fire
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    {
        let f = failed.lock().unwrap();
        assert!(
            !f.is_empty(),
            "on_request_failed handler should have fired, got: {:?}",
            f
        );
        assert!(
            f[0].starts_with("FAIL GET"),
            "Failed event should contain method: {}",
            f[0]
        );
    }

    browser.close().await.expect("Failed to close browser");
}

// ---------------------------------------------------------------------------
// Context handlers fire alongside page handlers (not instead of them)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_context_and_page_handlers_both_fire() {
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

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    let ctx_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let page_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

    let ctx_events2 = ctx_events.clone();
    context
        .on_request(move |request| {
            let evts = ctx_events2.clone();
            async move {
                evts.lock().unwrap().push(format!("CTX {}", request.url()));
                Ok(())
            }
        })
        .await
        .expect("Failed to register context on_request handler");

    let page = context.new_page().await.expect("Failed to create page");

    let page_events2 = page_events.clone();
    page.on_request(move |request| {
        let evts = page_events2.clone();
        async move {
            evts.lock().unwrap().push(format!("PAGE {}", request.url()));
            Ok(())
        }
    })
    .await
    .expect("Failed to register page on_request handler");

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    // Give the async events a moment to fire
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    {
        let ctx = ctx_events.lock().unwrap();
        let page = page_events.lock().unwrap();

        assert!(
            !ctx.is_empty(),
            "Context on_request handler should have fired"
        );
        assert!(
            !page.is_empty(),
            "Page on_request handler should have fired"
        );
    }

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
