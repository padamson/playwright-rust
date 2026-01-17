//! Tests for chunked message reading in transport layer
//!
//! These tests verify that the transport correctly handles large messages
//! by reading them in chunks rather than all at once.
//!
//! Note: While we cannot directly test memory usage in unit tests,
//! these tests verify correctness of chunked reading for various message sizes.
//! The implementation should read messages >32KB in 32KB chunks to reduce
//! peak memory usage.

use serde_json::json;
use tokio::io::AsyncWriteExt;

mod common;

/// Test that small messages (< 32KB) are handled correctly
#[tokio::test]
async fn test_small_message_reading() {
    common::init_tracing();
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
    common::init_tracing();
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
    common::init_tracing();
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
    common::init_tracing();
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
    common::init_tracing();
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
    common::init_tracing();
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
