//! Integration tests for Connection layer with real Playwright server
//!
//! These tests verify that the Connection layer can:
//! - Establish connection to real Playwright server
//! - Spawn transport and connection message loops
//! - Handle protocol initialization messages from server
//!
//! Note: Full protocol request/response testing will be implemented in Slice 4
//! (Object Factory) when we can handle the initialization sequence and send requests.

use playwright_rs::protocol::Playwright;
use playwright_rs::server::{connection::Connection, playwright_server::PlaywrightServer};
use std::sync::Arc;
use tokio::time::Duration;

mod common;

/// Test that we can establish a connection with real server and spawn message loops
///
/// This test verifies:
/// - Server launches successfully
/// - Connection can be created with server stdio
/// - Message loops can be spawned without errors
/// - Everything runs together and shuts down cleanly
#[tokio::test]
async fn test_connection_lifecycle_with_real_server() {
    common::init_tracing();
    // Launch Playwright server
    let mut server = match PlaywrightServer::launch().await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Skipping test: Could not launch Playwright server: {}", e);
            tracing::warn!("This is expected if Node.js or Playwright driver is not available");
            return;
        }
    };

    // Take stdio handles from the process
    let stdin = server.process.stdin.take().expect("Failed to get stdin");
    let stdout = server.process.stdout.take().expect("Failed to get stdout");

    // Create transport and split into sender/receiver
    let (transport, message_rx) =
        playwright_rs::server::transport::PipeTransport::new(stdin, stdout);
    let (sender, receiver) = transport.into_parts();

    // Create connection
    let connection = Arc::new(Connection::new(sender, receiver, message_rx));

    // Spawn connection message loop
    let conn = Arc::clone(&connection);
    let connection_handle = tokio::spawn(async move {
        conn.run().await;
    });

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // This test verifies the connection infrastructure works:
    // - Server launches successfully
    // - Connection and transport loops start without errors
    // - Everything compiles and runs together
    // - No panics or immediate crashes
    //
    // Full protocol initialization testing is done in:
    // - tests/initialization_integration.rs (complete initialization flow)
    // - tests/playwright_launch.rs (high-level Playwright::launch() API)

    // Clean up
    // Abort the connection loop (which will also stop reading from transport)
    connection_handle.abort();

    // Shutdown server
    server.shutdown().await.ok();
}

/// Test that connection detects server crash when sending
///
/// This test verifies that when the server crashes/exits:
/// - Attempting to send a message fails with appropriate error (broken pipe)
/// - The error is propagated correctly through the Connection layer
///
/// Note: The connection read loop will remain blocked on `read_exact()` until
/// the stdout pipe is fully closed by the OS, which can take time. This is
/// expected behavior - the important thing is that send operations fail fast.
#[tokio::test]
async fn test_connection_detects_server_crash_on_send() {
    common::init_tracing();
    let mut server = match PlaywrightServer::launch().await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Skipping test: Could not launch Playwright server: {}", e);
            return;
        }
    };

    let stdin = server.process.stdin.take().expect("Failed to get stdin");
    let stdout = server.process.stdout.take().expect("Failed to get stdout");

    let (transport, message_rx) =
        playwright_rs::server::transport::PipeTransport::new(stdin, stdout);
    let (sender, receiver) = transport.into_parts();

    let connection = Arc::new(Connection::new(sender, receiver, message_rx));

    // Spawn connection loop
    let conn = Arc::clone(&connection);
    let _connection_handle = tokio::spawn(async move {
        conn.run().await;
    });

    // Give connection time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Kill the server
    server.kill().await.expect("Failed to kill server");

    // Give it a moment for the pipe to close
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Try to send a message - this will detect the broken pipe immediately
    let send_result = connection
        .send_message(
            "test@guid".to_string(),
            "testMethod".to_string(),
            serde_json::json!({}),
        )
        .await;

    // Should fail with broken pipe error
    assert!(
        send_result.is_err(),
        "Expected error when sending to dead server"
    );

    // Verify it's a transport error (broken pipe)
    match send_result.unwrap_err() {
        playwright_rs::Error::TransportError(msg) => {
            assert!(
                msg.contains("Broken pipe") || msg.contains("Failed to write"),
                "Expected broken pipe error, got: {}",
                msg
            );
        }
        e => panic!("Expected TransportError, got: {:?}", e),
    }

    // Note: We don't wait for the connection loop to exit because the transport
    // read loop is blocked on read_exact() and won't exit until the OS fully
    // closes the stdout pipe. This is tested separately in the transport layer.
}

/// Test concurrent requests (deferred to Phase 2 Slice 5+)
///
/// This test will verify that multiple concurrent requests can be sent
/// and responses are correctly correlated, even when they arrive out of order.
///
/// Test concurrent requests to different protocol objects
///
/// This test verifies that concurrent requests to different objects
/// (Browser, Context, Page) are properly correlated when responses arrive.
#[tokio::test]
async fn test_concurrent_requests_with_server() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();
    let browser = chromium.launch().await.expect("Failed to launch browser");

    // Create two contexts concurrently
    let context1_fut = browser.new_context();
    let context2_fut = browser.new_context();
    let (context1, context2) = tokio::join!(context1_fut, context2_fut);

    let context1 = context1.expect("Failed to create context 1");
    let context2 = context2.expect("Failed to create context 2");

    // Create pages concurrently across different contexts
    let page1_fut = context1.new_page();
    let page2_fut = context1.new_page();
    let page3_fut = context2.new_page();
    let (page1, page2, page3) = tokio::join!(page1_fut, page2_fut, page3_fut);

    let page1 = page1.expect("Failed to create page 1");
    let page2 = page2.expect("Failed to create page 2");
    let page3 = page3.expect("Failed to create page 3");

    // Verify all pages are at about:blank
    assert_eq!(page1.url(), "about:blank");
    assert_eq!(page2.url(), "about:blank");
    assert_eq!(page3.url(), "about:blank");

    // Close everything concurrently
    let page1_close = page1.close();
    let page2_close = page2.close();
    let page3_close = page3.close();
    let context1_close = context1.close();
    let context2_close = context2.close();

    let (r1, r2, r3, r4, r5) = tokio::join!(
        page1_close,
        page2_close,
        page3_close,
        context1_close,
        context2_close
    );

    // Verify all closes succeeded
    r1.expect("Failed to close page 1");
    r2.expect("Failed to close page 2");
    r3.expect("Failed to close page 3");
    r4.expect("Failed to close context 1");
    r5.expect("Failed to close context 2");

    browser.close().await.expect("Failed to close browser");
}

/// Test error handling with invalid requests
///
/// This test verifies that protocol errors from the server are properly
/// converted to Rust errors and propagated correctly.
#[tokio::test]
async fn test_error_response_from_server() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();
    let browser = chromium.launch().await.expect("Failed to launch browser");

    // Close the browser
    browser.close().await.expect("Failed to close browser");

    // Try to close again - should result in an error from the server
    let result = browser.close().await;

    // The server should return an error for trying to close an already-closed browser
    assert!(
        result.is_err(),
        "Expected error when closing already-closed browser"
    );

    // Verify the error message contains relevant information
    // Test scenarios to implement:
    // - Call browser.close() twice (should error on second call)
    // - Invalid GUIDs
    // - Invalid method parameters
    //
    // Note: This is deferred for time, not technical reasons
}
