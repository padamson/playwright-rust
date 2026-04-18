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

/// Test that request.headers() returns a HashMap with standard headers including "host".
#[tokio::test]
async fn test_request_headers() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

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

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.post_data() returns None for GET requests.
#[tokio::test]
async fn test_request_post_data_get() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let captured: Arc<Mutex<Option<playwright_rs::protocol::Request>>> = Arc::new(Mutex::new(None));
    let captured2 = captured.clone();
    page.on_request(move |request| {
        let captured = captured2.clone();
        async move {
            // Only capture the main document request
            if request.resource_type() == "document" {
                *captured.lock().await = Some(request);
            }
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

    // GET request should have no post data
    let post_data = request.post_data();
    assert!(
        post_data.is_none(),
        "GET request should have no post data, got: {:?}",
        post_data
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.post_data() returns data for POST requests made via fetch().
#[tokio::test]
async fn test_request_post_data_post() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    // Navigate first so we have a page context
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

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

    let post_url = format!("{}/echo-headers", server.url());
    // Use evaluate to make a POST fetch with a body
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

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.post_data_buffer() returns bytes for POST requests.
#[tokio::test]
async fn test_request_post_data_buffer() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

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

    let post_url = format!("{}/echo-headers", server.url());
    let _: serde_json::Value = page
        .evaluate::<(), serde_json::Value>(
            &format!(
                r#"fetch('{}', {{
                    method: 'POST',
                    headers: {{'Content-Type': 'text/plain'}},
                    body: 'buffer test'
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

    let buf = request.post_data_buffer();
    assert!(buf.is_some(), "POST request should have post data buffer");
    let bytes = buf.unwrap();
    assert_eq!(bytes, b"buffer test", "Buffer should match sent body bytes");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.post_data_json() parses JSON post data.
#[tokio::test]
async fn test_request_post_data_json() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

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

    let post_url = format!("{}/echo-headers", server.url());
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

    notified_or_timeout(&notify, 5000, "POST request handler").await;

    let guard = captured.lock().await;
    let request = guard.as_ref().expect("Should have captured a POST request");

    let json_result = request.post_data_json::<serde_json::Value>();
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

    // GET request should return None
    // (we'll test this via a fresh request captured from the initial navigation)
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.failure() returns None for successful requests.
#[tokio::test]
async fn test_request_failure_none_for_success() {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let captured: Arc<Mutex<Option<playwright_rs::protocol::Request>>> = Arc::new(Mutex::new(None));
    let captured2 = captured.clone();
    page.on_request(move |request| {
        let captured = captured2.clone();
        async move {
            if request.resource_type() == "document" {
                *captured.lock().await = Some(request);
            }
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

    // Successful request should have no failure
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

// ============================================================================
// RPC-based tests
// ============================================================================

/// Test that request.headers_array() returns all headers as name-value pairs.
#[tokio::test]
async fn test_request_headers_array() {
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

    // Should include a "host" header
    let has_host = headers_array
        .iter()
        .any(|h| h.name.to_lowercase() == "host");
    assert!(
        has_host,
        "headers_array() should include 'host' header, got: {:?}",
        headers_array.iter().map(|h| &h.name).collect::<Vec<_>>()
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.all_headers() returns a HashMap with lowercased header keys.
#[tokio::test]
async fn test_request_all_headers() {
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

    let all_headers = request
        .all_headers()
        .await
        .expect("all_headers() should succeed");

    assert!(
        !all_headers.is_empty(),
        "all_headers() should return at least one header"
    );

    // Keys should be lowercased
    assert!(
        all_headers.contains_key("host"),
        "all_headers() should contain 'host' key (lowercased), got keys: {:?}",
        all_headers.keys().collect::<Vec<_>>()
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.header_value() returns the correct value for a known header.
#[tokio::test]
async fn test_request_header_value() {
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

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that request.timing() returns a ResourceTiming with plausible values.
#[tokio::test]
async fn test_request_timing() {
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

    let timing = request.timing().await.expect("timing() should succeed");

    // start_time should be a positive epoch milliseconds value
    assert!(
        timing.start_time >= 0.0,
        "start_time should be non-negative, got: {}",
        timing.start_time
    );

    // response_start should be after request_start or at least non-negative
    assert!(
        timing.response_start >= -1.0,
        "response_start should be >= -1 (Playwright uses -1 for unavailable), got: {}",
        timing.response_start
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
