//! Windows-specific cleanup tests
//!
//! These tests verify that stdio pipes and processes clean up properly on Windows
//! without hanging. This addresses the Phase 1 deferred issue where integration tests
//! would hang on Windows due to stdio pipe cleanup problems.

use playwright_rs::server::{playwright_server::PlaywrightServer, transport::PipeTransport};
use std::time::Duration;

/// Test that server shutdown doesn't hang on Windows
///
/// This test launches a Playwright server and verifies that:
/// 1. The server starts successfully
/// 2. The server can be shut down without hanging
/// 3. All stdio pipes are closed properly
/// 4. The process terminates within a reasonable timeout
#[tokio::test]
async fn test_server_shutdown_no_hang() {
    // Launch server
    let server = match PlaywrightServer::launch().await {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Skipping test - Playwright not available");
            return;
        }
    };

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Shutdown with timeout - should complete without hanging
    let shutdown_result = tokio::time::timeout(Duration::from_secs(5), server.shutdown()).await;

    assert!(
        shutdown_result.is_ok(),
        "Shutdown timed out after 5 seconds - Windows stdio cleanup hanging"
    );

    assert!(shutdown_result.unwrap().is_ok(), "Shutdown returned error");
}

/// Test that repeated launch/shutdown cycles don't leak resources
///
/// This test verifies that multiple server lifecycles work correctly
/// and don't accumulate hanging processes or leaked file handles.
#[tokio::test]
async fn test_repeated_server_lifecycle() {
    for i in 0..3 {
        eprintln!("Iteration {}", i + 1);

        let server = match PlaywrightServer::launch().await {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test - Playwright not available");
                return;
            }
        };

        tokio::time::sleep(Duration::from_millis(50)).await;

        let shutdown_result = tokio::time::timeout(Duration::from_secs(5), server.shutdown()).await;

        assert!(
            shutdown_result.is_ok(),
            "Shutdown timed out on iteration {}",
            i + 1
        );
    }
}

/// Test that kill() method completes without hanging
///
/// Tests the force-kill path which should be even more robust
/// than graceful shutdown.
#[tokio::test]
async fn test_server_kill_no_hang() {
    let server = match PlaywrightServer::launch().await {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Skipping test - Playwright not available");
            return;
        }
    };

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Kill with timeout
    let kill_result = tokio::time::timeout(Duration::from_secs(5), server.kill()).await;

    assert!(
        kill_result.is_ok(),
        "Kill timed out after 5 seconds - Windows stdio cleanup hanging"
    );

    assert!(kill_result.unwrap().is_ok(), "Kill returned error");
}

/// Test that connection cleanup doesn't hang when server dies
///
/// This tests the cleanup path when stdin/stdout are taken from the process
/// and used in a transport layer.
#[tokio::test]
async fn test_connection_cleanup_no_hang() {
    let mut server = match PlaywrightServer::launch().await {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Skipping test - Playwright not available");
            return;
        }
    };

    // Take stdio handles
    let stdin = server.process.stdin.take().expect("stdin should exist");
    let stdout = server.process.stdout.take().expect("stdout should exist");

    // Create transport
    let (transport, mut _message_rx) = PipeTransport::new(stdin, stdout);

    // Split transport
    let (mut _stdin_handle, _receiver) = transport.into_parts();

    // Now kill the server process - this simulates unexpected termination
    let kill_result = tokio::time::timeout(Duration::from_secs(5), server.process.kill()).await;

    assert!(
        kill_result.is_ok(),
        "Process kill timed out - Windows stdio cleanup hanging"
    );

    // Drop handles - this should complete without hanging
    drop(_stdin_handle);
    drop(_receiver);
    drop(_message_rx);

    // Wait for process to exit with timeout
    let wait_result = tokio::time::timeout(Duration::from_secs(2), server.process.wait()).await;

    assert!(
        wait_result.is_ok(),
        "Process wait timed out - Windows cleanup hanging"
    );
}
