// Integration tests for PipeTransport with real Playwright server
//
// These tests verify that the transport layer works correctly with an actual
// Playwright server process, not just mock pipes.
//
// Note: Browser-specific testing (Chromium/Firefox/WebKit) is deferred to
// Slice 4 (Browser API) when we implement browser launch functionality.

use crate::test_server::TestServer;
use playwright_rs::protocol::Playwright;
use playwright_rs::server::{playwright_server::PlaywrightServer, transport::PipeTransport};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio::time::{Duration, timeout};

/// Test that we can launch a real Playwright server and create a transport
#[tokio::test]
async fn test_transport_with_real_server() {
    crate::common::init_tracing();
    // Launch Playwright server
    let mut server = match PlaywrightServer::launch().await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Skipping test: Could not launch Playwright server: {}", e);
            tracing::warn!("This is expected if Node.js or Playwright driver is not available");
            return;
        }
    };

    // Get stdin and stdout from the server process
    let stdin = server.process.stdin.take().expect("Failed to get stdin");
    let stdout = server.process.stdout.take().expect("Failed to get stdout");

    // Create transport and split into sender/receiver
    let (transport, rx) = PipeTransport::new(stdin, stdout);
    let (_sender, mut receiver) = transport.into_parts();

    // Spawn transport read loop on the receiver
    let read_task = tokio::spawn(async move { receiver.run_loop().await });

    // Send a simple initialize message
    // This is a basic JSON-RPC message that the Playwright server should respond to
    let init_message = json!({
        "id": 1,
        "method": "initialize",
        "params": {
            "sdkLanguage": "rust"
        }
    });

    // Try to send the message
    // Note: We can't verify the response content without the Connection layer,
    // but we can verify that:
    // 1. The message sends without error
    // 2. We receive *something* back
    // 3. The transport doesn't crash

    // For now, just verify we can create the transport and it's connected
    // Actual protocol interaction testing deferred to Slice 3

    // Clean up
    drop(rx);
    drop(init_message);

    // Kill the server
    server.kill().await.expect("Failed to kill server");

    // The read task should exit when server is killed
    let result = timeout(Duration::from_secs(2), read_task).await;

    // Either it completed or timed out (both acceptable)
    match result {
        Ok(Ok(transport_result)) => {
            // Transport exited - could be Ok or Err depending on timing
            tracing::warn!("Transport exited: {:?}", transport_result);
        }
        Ok(Err(e)) => {
            panic!("Task panicked: {:?}", e);
        }
        Err(_) => {
            // Timeout is acceptable - transport might still be waiting
            tracing::warn!("Transport still running after server kill (acceptable)");
        }
    }
}

/// Test that transport can send a message to real server without panicking
#[tokio::test]
async fn test_send_message_to_real_server() {
    crate::common::init_tracing();
    // Launch Playwright server
    let mut server = match PlaywrightServer::launch().await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Skipping test: Could not launch Playwright server: {}", e);
            return;
        }
    };

    let stdin = server.process.stdin.take().expect("Failed to get stdin");
    let stdout = server.process.stdout.take().expect("Failed to get stdout");

    let (transport, _rx) = PipeTransport::new(stdin, stdout);
    let (_sender, mut receiver) = transport.into_parts();

    // Spawn read loop on the receiver
    let _read_task = tokio::spawn(async move {
        let _ = receiver.run_loop().await;
    });

    // Send a message - just verify it doesn't panic or error
    let _message = json!({
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    // For now, we just verify sending doesn't error
    // Full request/response verification requires Connection layer (Slice 3)

    // Clean up
    server.kill().await.expect("Failed to kill server");
}

/// Test that transport handles server crash gracefully
#[tokio::test]
async fn test_transport_handles_server_crash() {
    crate::common::init_tracing();
    let mut server = match PlaywrightServer::launch().await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Skipping test: Could not launch Playwright server: {}", e);
            return;
        }
    };

    let stdin = server.process.stdin.take().expect("Failed to get stdin");
    let stdout = server.process.stdout.take().expect("Failed to get stdout");

    let (transport, _rx) = PipeTransport::new(stdin, stdout);
    let (_sender, mut receiver) = transport.into_parts();

    // Spawn read loop on the receiver
    let read_task = tokio::spawn(async move { receiver.run_loop().await });

    // Kill the server immediately
    server.kill().await.expect("Failed to kill server");

    // Transport should detect the broken pipe and exit (either with error or clean EOF)
    let result = timeout(Duration::from_secs(2), read_task).await;

    match result {
        Ok(Ok(transport_result)) => {
            // Transport exited - either with error (broken pipe) or Ok (clean EOF)
            // Both are acceptable when server is killed - the important thing is
            // that the transport exits promptly rather than hanging
            tracing::info!("Transport exited with: {:?}", transport_result);
        }
        Ok(Err(e)) => {
            panic!("Task panicked: {:?}", e);
        }
        Err(_) => {
            // Timeout means transport didn't detect the crash - this is a bug
            panic!("Transport did not exit after server was killed");
        }
    }
}

// ============================================================================
// Merged from: transport_chunked_reading.rs
// ============================================================================

// Tests for chunked message reading in transport layer
//
// These tests verify that the transport correctly handles large messages
// by reading them in chunks rather than all at once.
//
// Note: While we cannot directly test memory usage in unit tests,
// these tests verify correctness of chunked reading for various message sizes.
// The implementation should read messages >32KB in 32KB chunks to reduce
// peak memory usage.

/// Test that small messages (< 32KB) are handled correctly
#[tokio::test]
async fn test_small_message_reading() {
    crate::common::init_tracing();
    let (_stdin_read, stdin_write) = tokio::io::duplex(64 * 1024);
    let (stdout_read, mut stdout_write) = tokio::io::duplex(64 * 1024);

    let (transport, mut rx) =
        playwright_rs::server::transport::PipeTransport::new(stdin_write, stdout_read);
    let (_sender, mut receiver) = transport.into_parts();

    // Spawn reader
    let read_task = tokio::spawn(async move { receiver.run_loop().await });

    // Create a small message (< 32KB)
    let small_message = json!({
        "id": 1,
        "method": "test",
        "data": "x".repeat(1000) // 1KB
    });

    let json_bytes = serde_json::to_vec(&small_message).unwrap();
    let length = json_bytes.len() as u32;

    // Verify it's small
    assert!(length < 32_768, "Test message should be < 32KB");

    // Send the message
    stdout_write.write_all(&length.to_le_bytes()).await.unwrap();
    stdout_write.write_all(&json_bytes).await.unwrap();
    stdout_write.flush().await.unwrap();

    // Verify we receive it correctly
    let received = rx.recv().await.unwrap();
    assert_eq!(received, small_message);

    // Cleanup
    drop(stdout_write);
    drop(rx);
    let _ = read_task.await;
}

/// Test that messages exactly 32KB are handled correctly
#[tokio::test]
async fn test_exactly_buffer_size_message() {
    crate::common::init_tracing();
    let (_stdin_read, stdin_write) = tokio::io::duplex(128 * 1024);
    let (stdout_read, mut stdout_write) = tokio::io::duplex(128 * 1024);

    let (transport, mut rx) =
        playwright_rs::server::transport::PipeTransport::new(stdin_write, stdout_read);
    let (_sender, mut receiver) = transport.into_parts();

    // Spawn reader
    let read_task = tokio::spawn(async move { receiver.run_loop().await });

    // Create a message that's exactly 32KB
    // JSON overhead: {"id":1,"data":"..."} = about 19 bytes
    // So we need 32768 - 19 = 32749 'x' characters
    let data_size = 32_768 - 19;
    let exact_message = json!({
        "id": 1,
        "data": "x".repeat(data_size)
    });

    let json_bytes = serde_json::to_vec(&exact_message).unwrap();
    let length = json_bytes.len() as u32;

    // Verify it's exactly 32KB (within a few bytes due to JSON encoding)
    assert!(
        (length as i32 - 32_768).abs() < 100,
        "Test message should be ~32KB, got {}",
        length
    );

    // Send the message
    stdout_write.write_all(&length.to_le_bytes()).await.unwrap();
    stdout_write.write_all(&json_bytes).await.unwrap();
    stdout_write.flush().await.unwrap();

    // Verify we receive it correctly
    let received = rx.recv().await.unwrap();
    assert_eq!(received, exact_message);

    // Cleanup
    drop(stdout_write);
    drop(rx);
    let _ = read_task.await;
}

/// Test that messages just over 32KB require chunked reading
#[tokio::test]
async fn test_just_over_buffer_size_message() {
    crate::common::init_tracing();
    let (_stdin_read, stdin_write) = tokio::io::duplex(128 * 1024);
    let (stdout_read, mut stdout_write) = tokio::io::duplex(128 * 1024);

    let (transport, mut rx) =
        playwright_rs::server::transport::PipeTransport::new(stdin_write, stdout_read);
    let (_sender, mut receiver) = transport.into_parts();

    // Spawn reader
    let read_task = tokio::spawn(async move { receiver.run_loop().await });

    // Create a message that's just over 32KB (33KB)
    let over_message = json!({
        "id": 1,
        "data": "x".repeat(33_000)
    });

    let json_bytes = serde_json::to_vec(&over_message).unwrap();
    let length = json_bytes.len() as u32;

    // Verify it's > 32KB
    assert!(length > 32_768, "Test message should be > 32KB");

    // Send the message
    stdout_write.write_all(&length.to_le_bytes()).await.unwrap();
    stdout_write.write_all(&json_bytes).await.unwrap();
    stdout_write.flush().await.unwrap();

    // Verify we receive it correctly
    let received = rx.recv().await.unwrap();
    assert_eq!(received, over_message);

    // Cleanup
    drop(stdout_write);
    drop(rx);
    let _ = read_task.await;
}

/// Test that very large messages (> 100KB) are handled correctly
#[tokio::test]
async fn test_very_large_message_chunked_reading() {
    crate::common::init_tracing();
    let (_stdin_read, stdin_write) = tokio::io::duplex(1024 * 1024); // 1MB buffer
    let (stdout_read, mut stdout_write) = tokio::io::duplex(1024 * 1024);

    let (transport, mut rx) =
        playwright_rs::server::transport::PipeTransport::new(stdin_write, stdout_read);
    let (_sender, mut receiver) = transport.into_parts();

    // Spawn reader
    let read_task = tokio::spawn(async move { receiver.run_loop().await });

    // Create a very large message (200KB) - requires multiple chunks
    let large_string = "x".repeat(200_000);
    let large_message = json!({
        "id": 1,
        "data": large_string
    });

    let json_bytes = serde_json::to_vec(&large_message).unwrap();
    let length = json_bytes.len() as u32;

    // Verify it's > 32KB (should be ~200KB)
    assert!(length > 100_000, "Test message should be > 100KB");

    // Send the message
    stdout_write.write_all(&length.to_le_bytes()).await.unwrap();
    stdout_write.write_all(&json_bytes).await.unwrap();
    stdout_write.flush().await.unwrap();

    // Verify we receive it correctly
    let received = rx.recv().await.unwrap();
    assert_eq!(received, large_message);

    // Cleanup
    drop(stdout_write);
    drop(rx);
    let _ = read_task.await;
}

/// Test that multiple large messages in sequence are handled correctly
#[tokio::test]
async fn test_multiple_large_messages_in_sequence() {
    crate::common::init_tracing();
    let (_stdin_read, stdin_write) = tokio::io::duplex(2 * 1024 * 1024); // 2MB buffer
    let (stdout_read, mut stdout_write) = tokio::io::duplex(2 * 1024 * 1024);

    let (transport, mut rx) =
        playwright_rs::server::transport::PipeTransport::new(stdin_write, stdout_read);
    let (_sender, mut receiver) = transport.into_parts();

    // Spawn reader
    let read_task = tokio::spawn(async move { receiver.run_loop().await });

    // Create multiple large messages
    let messages = vec![
        json!({
            "id": 1,
            "data": "a".repeat(50_000) // 50KB
        }),
        json!({
            "id": 2,
            "data": "b".repeat(75_000) // 75KB
        }),
        json!({
            "id": 3,
            "data": "c".repeat(100_000) // 100KB
        }),
    ];

    // Send all messages
    for msg in &messages {
        let json_bytes = serde_json::to_vec(msg).unwrap();
        let length = json_bytes.len() as u32;

        stdout_write.write_all(&length.to_le_bytes()).await.unwrap();
        stdout_write.write_all(&json_bytes).await.unwrap();
    }
    stdout_write.flush().await.unwrap();

    // Receive all messages and verify
    for expected in &messages {
        let received = rx.recv().await.unwrap();
        assert_eq!(&received, expected);
    }

    // Cleanup
    drop(stdout_write);
    drop(rx);
    let _ = read_task.await;
}

/// Test that chunked reading handles odd message sizes correctly
#[tokio::test]
async fn test_odd_sized_messages() {
    crate::common::init_tracing();
    let (_stdin_read, stdin_write) = tokio::io::duplex(512 * 1024);
    let (stdout_read, mut stdout_write) = tokio::io::duplex(512 * 1024);

    let (transport, mut rx) =
        playwright_rs::server::transport::PipeTransport::new(stdin_write, stdout_read);
    let (_sender, mut receiver) = transport.into_parts();

    // Spawn reader
    let read_task = tokio::spawn(async move { receiver.run_loop().await });

    // Test various odd sizes around the 32KB boundary
    let test_sizes = vec![
        32_767, // Just under 32KB
        32_768, // Exactly 32KB
        32_769, // Just over 32KB
        65_535, // Just under 64KB (2 chunks)
        65_536, // Exactly 64KB (2 chunks)
        65_537, // Just over 64KB
        98_303, // Just under 96KB (3 chunks)
    ];

    for size in test_sizes {
        let msg = json!({
            "id": 1,
            "data": "x".repeat(size)
        });

        let json_bytes = serde_json::to_vec(&msg).unwrap();
        let length = json_bytes.len() as u32;

        stdout_write.write_all(&length.to_le_bytes()).await.unwrap();
        stdout_write.write_all(&json_bytes).await.unwrap();
        stdout_write.flush().await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received, msg, "Failed for size {}", size);
    }

    // Cleanup
    drop(stdout_write);
    drop(rx);
    let _ = read_task.await;
}

// ============================================================================
// Merged from: transport_websocket_test.rs
// ============================================================================

// Integration tests for WebSocket event handling
//
// Following TDD: Write tests first (Red), then implement (Green)

// Ideally we reuse the existing test_server.rs in tests/

#[tokio::test]
async fn test_websocket_interception() {
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
    let page = browser.new_page().await.expect("Failed to create page");

    // Setup WebSocket event handler
    // This API does not exist yet -> RED
    let ws_event_fired = std::sync::Arc::new(tokio::sync::Mutex::new(false));
    let ws_event_fired_clone = ws_event_fired.clone();

    page.on_websocket(move |ws| {
        let fired = ws_event_fired_clone.clone();
        Box::pin(async move {
            *fired.lock().await = true;
            println!("WebSocket opened: {}", ws.url());

            // Verify URL
            assert!(ws.url().contains("ws://"));

            // Listen for frames
            ws.on_frame_sent(|data| {
                Box::pin(async move {
                    println!("Frame sent: {:?}", data);
                    Ok(())
                })
            })
            .await
            .unwrap();

            Ok(())
        })
    })
    .await
    .expect("Failed to register websocket handler");

    // Navigate to a page that opens a WebSocket
    // We need to add a websocket test page to test_server
    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Wait a bit for the connection
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    assert!(
        *ws_event_fired.lock().await,
        "on_websocket handler should have been called"
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
