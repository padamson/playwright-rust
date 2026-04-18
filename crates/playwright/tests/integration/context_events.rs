// across all pages in the context.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

use crate::test_server::TestServer;

/// Helper: await a Notify with a timeout, failing the test on timeout.
async fn notified_or_timeout(notify: &Notify, ms: u64, what: &str) {
    tokio::time::timeout(Duration::from_millis(ms), notify.notified())
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

// ---------------------------------------------------------------------------
// on_page
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_context_on_page() {
    let (_pw, browser, context) = crate::common::setup_context().await;

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

    // Set up waiter before the action so the handler is guaranteed to run
    // before the waiter resolves (dispatch awaits all handlers, then notifies).
    let waiter = context
        .expect_event("page", Some(5000.0))
        .await
        .expect("Failed to create page event waiter");

    let _page = context.new_page().await.expect("Failed to create page");

    waiter.wait().await.expect("page event did not fire");

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
    let (_pw, browser, context) = crate::common::setup_context().await;

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

    let waiter = context
        .expect_event("close", Some(5000.0))
        .await
        .expect("Failed to create close event waiter");

    context.close().await.expect("Failed to close context");

    waiter.wait().await.expect("close event did not fire");

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
    let (_pw, browser, context) = crate::common::setup_context().await;
    let server = TestServer::start().await;

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

    let waiter = context
        .expect_event("request", Some(5000.0))
        .await
        .expect("Failed to create request event waiter");

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    waiter.wait().await.expect("request event did not fire");

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
    let (_pw, browser, context) = crate::common::setup_context().await;
    let server = TestServer::start().await;

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

    let waiter = context
        .expect_event("response", Some(5000.0))
        .await
        .expect("Failed to create response event waiter");

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    waiter.wait().await.expect("response event did not fire");

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
    let (_pw, browser, context) = crate::common::setup_context().await;
    let server = TestServer::start().await;

    let finished: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let finished2 = finished.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    context
        .on_request_finished(move |request| {
            let fin = finished2.clone();
            let n = notify2.clone();
            async move {
                fin.lock().unwrap().push(format!("DONE {}", request.url()));
                n.notify_one();
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_request_finished handler");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    notified_or_timeout(&notify, 5000, "on_request_finished").await;

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
    let (_pw, browser, context) = crate::common::setup_context().await;

    let failed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let failed2 = failed.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    context
        .on_request_failed(move |request| {
            let f = failed2.clone();
            let n = notify2.clone();
            async move {
                f.lock()
                    .unwrap()
                    .push(format!("FAIL {} {}", request.method(), request.url()));
                n.notify_one();
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_request_failed handler");

    let page = context.new_page().await.expect("Failed to create page");

    // Navigate to a non-routable address to trigger a request failure
    let result = page.goto("http://localhost:1", None).await;
    assert!(result.is_err(), "Navigation to bad port should fail");

    notified_or_timeout(&notify, 5000, "on_request_failed").await;

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
    let (_pw, browser, context) = crate::common::setup_context().await;
    let server = TestServer::start().await;

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

    // Waiter fires after all handlers have completed
    let waiter = context
        .expect_event("request", Some(5000.0))
        .await
        .expect("Failed to create request event waiter");

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    waiter.wait().await.expect("request event did not fire");

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

// ---------------------------------------------------------------------------
// common::setup() smoke test
// ---------------------------------------------------------------------------

/// Verify the common::setup() helper works and returns a usable page.
#[tokio::test]
async fn test_common_setup_helper() {
    let (_pw, browser, page) = crate::common::setup().await;

    let title = page.title().await.expect("Failed to get title");
    assert!(title.is_empty() || !title.is_empty()); // just verify callable

    browser.close().await.expect("Failed to close browser");
}

// ---------------------------------------------------------------------------
// expect_page
// ---------------------------------------------------------------------------

/// Test that expect_page() resolves with the new Page when a page is created.
/// The waiter MUST be set up before the action that creates the page to avoid
/// a race condition.
#[tokio::test]
async fn test_context_expect_page() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    // Set up waiter BEFORE creating the page (critical to avoid race)
    let waiter = context
        .expect_page(None)
        .await
        .expect("Failed to create page waiter");

    // Now create the page to trigger the event
    let _created = context.new_page().await.expect("Failed to create page");

    // The waiter should resolve with the new page
    let received_page = waiter.wait().await.expect("expect_page waiter timed out");

    // Verify we got a real Page object (it should have a URL)
    let url = received_page.url();
    assert!(
        url == "about:blank" || url.is_empty(),
        "Expected new page URL to be about:blank, got: {url}"
    );

    browser.close().await.expect("Failed to close browser");
}

/// Test that expect_page() times out when no page is created within the timeout.
#[tokio::test]
async fn test_context_expect_page_timeout() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    // Use a very short timeout (100ms) — no page will be created
    let waiter = context
        .expect_page(Some(100.0))
        .await
        .expect("Failed to create page waiter");

    // Should timeout because no page is created
    let result = waiter.wait().await;
    assert!(
        result.is_err(),
        "expect_page should have timed out but succeeded"
    );

    // Verify it's a timeout error
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.to_lowercase().contains("timeout") || err_str.to_lowercase().contains("timed out"),
        "Expected timeout error, got: {err_str}"
    );

    browser.close().await.expect("Failed to close browser");
}

/// Test that expect_close() resolves when the context is closed.
#[tokio::test]
async fn test_context_expect_close() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    // Set up the close waiter BEFORE closing the context
    let waiter = context
        .expect_close(None)
        .await
        .expect("Failed to create close waiter");

    // Close the context to trigger the event
    context.close().await.expect("Failed to close context");

    // Waiter should resolve successfully
    waiter
        .wait()
        .await
        .expect("expect_close waiter should have resolved");

    browser.close().await.expect("Failed to close browser");
}

// ---------------------------------------------------------------------------
// on_dialog (context-level)
// ---------------------------------------------------------------------------

/// A dialog handler registered on the BrowserContext fires for dialogs triggered
/// from any page in that context.
#[tokio::test]
async fn test_context_on_dialog() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let dialog_messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let dialog_messages2 = dialog_messages.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    context
        .on_dialog(move |dialog| {
            let msgs = dialog_messages2.clone();
            let n = notify2.clone();
            async move {
                msgs.lock().unwrap().push(dialog.message().to_string());
                let result = dialog.accept(None).await;
                n.notify_one();
                result
            }
        })
        .await
        .expect("Failed to register on_dialog handler");

    let page = context.new_page().await.expect("Failed to create page");
    let _ = page.goto("about:blank", None).await;

    page.evaluate_expression(
        r#"
        const button = document.createElement('button');
        button.onclick = () => alert('Context dialog!');
        button.textContent = 'Alert';
        document.body.appendChild(button);
        "#,
    )
    .await
    .expect("evaluate_expression should succeed");

    let locator = page.locator("button").await;
    locator.click(None).await.expect("click should succeed");

    notified_or_timeout(&notify, 5000, "on_dialog").await;

    {
        let msgs = dialog_messages.lock().unwrap();
        assert_eq!(
            msgs.len(),
            1,
            "on_dialog context handler should fire once, got: {:?}",
            msgs
        );
        assert_eq!(
            msgs[0], "Context dialog!",
            "dialog message mismatch: {:?}",
            msgs
        );
    }

    browser.close().await.expect("Failed to close browser");
}
