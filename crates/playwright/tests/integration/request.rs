use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, Notify};

use crate::test_server::TestServer;

/// Helper: await a Notify with a timeout, failing the test on timeout.
async fn notified_or_timeout(notify: &Notify, ms: u64, what: &str) {
    tokio::time::timeout(Duration::from_millis(ms), notify.notified())
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

// ============================================================================
// Local read tests (no RPC needed)
// ============================================================================

/// Exercises local GET-request accessors in a single browser session:
/// headers(), post_data() for GET, and failure() for a successful request.
#[tokio::test]
async fn test_request_local_get_accessors() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // --- headers() returns a non-empty map with at least one standard browser header ---

    let captured: Arc<Mutex<Option<playwright_rs::protocol::Request>>> = Arc::new(Mutex::new(None));
    let captured2 = captured.clone();
    page.on_request(move |request| {
        let captured = captured2.clone();
        async move {
            *captured.lock().await = Some(request);
            Ok(())
        }
    })
    .await
    .expect("Failed to set request handler");

    let waiter = page
        .expect_event("request", Some(5000.0))
        .await
        .expect("Failed to create request event waiter");

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    waiter.wait().await.expect("request event did not fire");

    let guard = captured.lock().await;
    let request = guard.as_ref().expect("Should have captured a request");

    let headers = request.headers();
    assert!(
        !headers.is_empty(),
        "Request headers should not be empty, got 0 headers"
    );

    // The headers() method reads from the initializer which contains browser-set headers
    // (user-agent, accept, etc.) but NOT the "host" header (that's only in rawRequestHeaders).
    // Verify at least one well-known browser header is present.
    let has_browser_header = headers.keys().any(|k| {
        matches!(
            k.as_str(),
            "user-agent"
                | "accept"
                | "accept-encoding"
                | "accept-language"
                | "host"
                | "upgrade-insecure-requests"
                | "sec-ch-ua"
                | "sec-ch-ua-mobile"
                | "sec-ch-ua-platform"
        )
    });
    assert!(
        has_browser_header,
        "Request headers should contain at least one standard browser header, got: {:?}",
        headers.keys().collect::<Vec<_>>()
    );

    // --- post_data() returns None for GET requests ---
    // The captured request above is a document GET; post_data() should be None.
    let post_data = request.post_data();
    assert!(
        post_data.is_none(),
        "GET request should have no post data, got: {:?}",
        post_data
    );

    // --- failure() returns None for a successful request ---
    let failure = request.failure();
    assert!(
        failure.is_none(),
        "Successful request should have no failure, got: {:?}",
        failure
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.failure() returns Some(error_text) for failed requests.
#[tokio::test]
async fn test_request_failure_text() {
    let (_pw, browser, page) = crate::common::setup().await;

    let captured: Arc<Mutex<Option<playwright_rs::protocol::Request>>> = Arc::new(Mutex::new(None));
    let captured2 = captured.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();
    page.on_request_failed(move |request| {
        let captured = captured2.clone();
        let n = notify2.clone();
        async move {
            *captured.lock().await = Some(request);
            n.notify_one();
            Ok(())
        }
    })
    .await
    .expect("Failed to set request failed handler");

    // Navigate to an invalid address to trigger failure
    let _ = page.goto("http://localhost:1", None).await;

    notified_or_timeout(&notify, 5000, "request_failed handler").await;

    let guard = captured.lock().await;
    let request = guard
        .as_ref()
        .expect("Should have captured a failed request");

    let failure = request.failure();
    assert!(failure.is_some(), "Failed request should have failure text");
    let error_text = failure.unwrap();
    assert!(
        !error_text.is_empty(),
        "Failure text should not be empty, got: {:?}",
        error_text
    );

    browser.close().await.expect("Failed to close browser");
}

/// Exercises POST-request data accessors in a single browser session:
/// post_data(), post_data_buffer(), and post_data_json().
#[tokio::test]
async fn test_request_post_data_methods() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // Navigate first so we have a page context
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    let post_url = format!("{}/echo-headers", server.url());

    // --- post_data() returns the raw body string for POST requests ---

    let captured: Arc<Mutex<Option<playwright_rs::protocol::Request>>> = Arc::new(Mutex::new(None));
    let captured2 = captured.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();
    page.on_request(move |request| {
        let captured = captured2.clone();
        let n = notify2.clone();
        async move {
            if request.method() == "POST" {
                *captured.lock().await = Some(request);
                n.notify_one();
            }
            Ok(())
        }
    })
    .await
    .expect("Failed to set request handler");

    let _: serde_json::Value = page
        .evaluate::<(), serde_json::Value>(
            &format!(
                r#"fetch('{}', {{
                    method: 'POST',
                    headers: {{'Content-Type': 'text/plain'}},
                    body: 'hello playwright'
                }}).then(r => r.text())"#,
                post_url
            ),
            None,
        )
        .await
        .expect("fetch should succeed");

    notified_or_timeout(&notify, 5000, "POST request handler").await;

    let guard = captured.lock().await;
    let request = guard.as_ref().expect("Should have captured a POST request");

    let post_data = request.post_data();
    assert!(post_data.is_some(), "POST request should have post data");
    let data = post_data.unwrap();
    assert_eq!(
        data, "hello playwright",
        "Post data should match sent body, got: {:?}",
        data
    );

    // --- post_data_buffer() returns the body as raw bytes ---
    let buf = request.post_data_buffer();
    assert!(buf.is_some(), "POST request should have post data buffer");
    let bytes = buf.unwrap();
    assert_eq!(
        bytes, b"hello playwright",
        "Buffer should match sent body bytes"
    );

    drop(guard);

    // --- post_data_json() parses a JSON body ---

    let captured_json: Arc<Mutex<Option<playwright_rs::protocol::Request>>> =
        Arc::new(Mutex::new(None));
    let captured_json2 = captured_json.clone();
    let notify_json = Arc::new(Notify::new());
    let notify_json2 = notify_json.clone();
    page.on_request(move |request| {
        let captured = captured_json2.clone();
        let n = notify_json2.clone();
        async move {
            if request.method() == "POST" {
                *captured.lock().await = Some(request);
                n.notify_one();
            }
            Ok(())
        }
    })
    .await
    .expect("Failed to set second request handler");

    let _: serde_json::Value = page
        .evaluate::<(), serde_json::Value>(
            &format!(
                r#"fetch('{}', {{
                    method: 'POST',
                    headers: {{'Content-Type': 'application/json'}},
                    body: JSON.stringify({{key: 'value', number: 42}})
                }}).then(r => r.text())"#,
                post_url
            ),
            None,
        )
        .await
        .expect("fetch should succeed");

    notified_or_timeout(&notify_json, 5000, "JSON POST request handler").await;

    let guard_json = captured_json.lock().await;
    let json_request = guard_json
        .as_ref()
        .expect("Should have captured a JSON POST request");

    let json_result = json_request.post_data_json::<serde_json::Value>();
    assert!(json_result.is_some(), "Should have JSON post data");
    let value = json_result.unwrap().expect("JSON parse should succeed");
    assert_eq!(
        value.get("key").and_then(|v| v.as_str()),
        Some("value"),
        "Should parse key correctly"
    );
    assert_eq!(
        value.get("number").and_then(|v| v.as_i64()),
        Some(42),
        "Should parse number correctly"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.existing_response() returns None initially and Some after the response event.
#[tokio::test]
async fn test_request_existing_response() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let captured_request: Arc<Mutex<Option<playwright_rs::protocol::Request>>> =
        Arc::new(Mutex::new(None));
    let captured_request2 = captured_request.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    // Capture the navigation request at request time (before response)
    page.on_request(move |request| {
        let captured = captured_request2.clone();
        let n = notify2.clone();
        async move {
            if request.is_navigation_request() {
                // At request time, existing_response() must return None
                let existing = request.existing_response();
                assert!(
                    existing.is_none(),
                    "existing_response() should be None before response fires"
                );
                *captured.lock().await = Some(request);
                n.notify_one();
            }
            Ok(())
        }
    })
    .await
    .expect("Failed to set request handler");

    let waiter = page
        .expect_event("response", Some(5000.0))
        .await
        .expect("Failed to create response event waiter");

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    // Wait for the request event first
    notified_or_timeout(&notify, 5000, "navigation request handler").await;

    // Wait for the response event to fire
    waiter.wait().await.expect("response event did not fire");

    // After response event, existing_response() should return Some
    let guard = captured_request.lock().await;
    let request = guard.as_ref().expect("Should have captured a request");
    let existing = request.existing_response();
    assert!(
        existing.is_some(),
        "existing_response() should be Some after the response event fires"
    );
    let resp = existing.unwrap();
    assert_eq!(resp.status(), 200, "Response status should be 200");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// RPC-based tests
// ============================================================================

/// Exercises RPC header and timing accessors in a single browser session:
/// headers_array(), all_headers(), header_value(), and timing().
#[tokio::test]
async fn test_request_rpc_header_and_timing_methods() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let captured: Arc<Mutex<Option<playwright_rs::protocol::Request>>> = Arc::new(Mutex::new(None));
    let captured2 = captured.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();
    page.on_request_finished(move |request| {
        let captured = captured2.clone();
        let n = notify2.clone();
        async move {
            if request.resource_type() == "document" {
                *captured.lock().await = Some(request);
                n.notify_one();
            }
            Ok(())
        }
    })
    .await
    .expect("Failed to set request handler");

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    notified_or_timeout(&notify, 5000, "request_finished handler").await;

    let guard = captured.lock().await;
    let request = guard
        .as_ref()
        .expect("Should have captured a finished request");

    // --- headers_array() returns all headers as name-value pairs ---
    let headers_array = request
        .headers_array()
        .await
        .expect("headers_array() should succeed");

    assert!(
        !headers_array.is_empty(),
        "headers_array() should return at least one header"
    );

    for entry in &headers_array {
        assert!(!entry.name.is_empty(), "Header name should not be empty");
    }

    let has_host = headers_array
        .iter()
        .any(|h| h.name.to_lowercase() == "host");
    assert!(
        has_host,
        "headers_array() should include 'host' header, got: {:?}",
        headers_array.iter().map(|h| &h.name).collect::<Vec<_>>()
    );

    // --- all_headers() returns a HashMap with lowercased header keys ---
    let all_headers = request
        .all_headers()
        .await
        .expect("all_headers() should succeed");

    assert!(
        !all_headers.is_empty(),
        "all_headers() should return at least one header"
    );

    assert!(
        all_headers.contains_key("host"),
        "all_headers() should contain 'host' key (lowercased), got keys: {:?}",
        all_headers.keys().collect::<Vec<_>>()
    );

    // --- header_value() returns the correct value for a known header ---

    // "host" header should be present
    let host = request
        .header_value("host")
        .await
        .expect("header_value() should succeed");
    assert!(host.is_some(), "header_value('host') should return a value");
    let host_val = host.unwrap();
    assert!(
        !host_val.is_empty(),
        "Host header value should not be empty"
    );

    // Case-insensitive: "Host" should match the same as "host"
    let host_upper = request
        .header_value("Host")
        .await
        .expect("header_value() should succeed with mixed case");
    assert_eq!(
        host_upper,
        Some(host_val.clone()),
        "header_value should be case-insensitive"
    );

    // Missing header should return None
    let missing = request
        .header_value("x-does-not-exist")
        .await
        .expect("header_value() should succeed even for missing headers");
    assert!(missing.is_none(), "Missing header should return None");

    // --- timing() returns a ResourceTiming with plausible values ---
    let timing = request.timing().await.expect("timing() should succeed");

    assert!(
        timing.start_time >= 0.0,
        "start_time should be non-negative, got: {}",
        timing.start_time
    );

    assert!(
        timing.response_start >= -1.0,
        "response_start should be >= -1 (Playwright uses -1 for unavailable), got: {}",
        timing.response_start
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
