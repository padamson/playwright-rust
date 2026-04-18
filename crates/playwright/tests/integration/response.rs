// Integration tests for Response body access and properties
//
// Tests for Response.body(), text(), json(), all_headers(),
// header_value(), and headers_array() methods.

use crate::test_server::TestServer;

// ============================================================================
// Response body access tests
// ============================================================================

/// Test that response.body() returns non-empty bytes for a page navigation.
#[tokio::test]
async fn test_response_body() {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

    let response = page
        .goto(&server.url(), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    let body = response.body().await.expect("body() should succeed");
    assert!(!body.is_empty(), "Response body should not be empty");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that response.text() returns the HTML content as a string.
#[tokio::test]
async fn test_response_text() {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

    let response = page
        .goto(&server.url(), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    let text = response.text().await.expect("text() should succeed");
    assert!(
        text.contains("<!DOCTYPE html>") || text.contains("<html"),
        "Response text should contain HTML: {}",
        &text[..text.len().min(200)]
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that response.json() parses a JSON endpoint response correctly.
#[tokio::test]
async fn test_response_json() {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

    // Navigate to the JSON endpoint served by TestServer
    let json_url = format!("{}/api/data.json", server.url());
    let response = page
        .goto(&json_url, None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    let value: serde_json::Value = response
        .json()
        .await
        .expect("json() should succeed for JSON endpoint");

    assert!(value.is_object(), "Response should parse as a JSON object");
    assert_eq!(
        value.get("status").and_then(|v| v.as_str()),
        Some("ok"),
        "JSON should contain status:ok"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that response.header_value() returns the content-type header.
#[tokio::test]
async fn test_response_header_value() {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

    let response = page
        .goto(&server.url(), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    let content_type = response
        .header_value("content-type")
        .await
        .expect("header_value() should succeed");

    assert!(
        content_type.is_some(),
        "content-type header should be present"
    );
    let ct = content_type.unwrap();
    assert!(
        ct.contains("text/html"),
        "content-type should contain text/html, got: {}",
        ct
    );

    // Test a header that doesn't exist
    let missing = response
        .header_value("x-does-not-exist")
        .await
        .expect("header_value() should succeed even for missing headers");
    assert!(missing.is_none(), "Missing header should return None");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that response.headers_array() returns all headers as name-value pairs.
#[tokio::test]
async fn test_response_headers_array() {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

    let response = page
        .goto(&server.url(), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    let headers = response
        .headers_array()
        .await
        .expect("headers_array() should succeed");

    assert!(
        !headers.is_empty(),
        "headers_array() should return at least one header"
    );

    // Check that each entry has both name and value fields
    for entry in &headers {
        assert!(!entry.name.is_empty(), "Header name should not be empty");
    }

    // Should include content-type
    let has_content_type = headers
        .iter()
        .any(|h| h.name.to_lowercase() == "content-type");
    assert!(
        has_content_type,
        "headers_array() should include content-type"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

/// Test that response.all_headers() returns a HashMap with headers merged (lowercased keys).
#[tokio::test]
async fn test_response_all_headers() {
    let (_pw, browser, page) = crate::common::setup().await;
    let server = TestServer::start().await;

    let response = page
        .goto(&server.url(), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    let all_headers = response
        .all_headers()
        .await
        .expect("all_headers() should succeed");

    assert!(
        !all_headers.is_empty(),
        "all_headers() should return at least one header"
    );

    // content-type key should be present (lowercased)
    assert!(
        all_headers.contains_key("content-type"),
        "all_headers() should contain content-type key, got keys: {:?}",
        all_headers.keys().collect::<Vec<_>>()
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
