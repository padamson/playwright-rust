//! Integration tests for remote browser connection via WebSocket
//!
//! These tests verify that `BrowserType::connect()` works with a real
//! Playwright browser server created via `chromium.launchServer()`.
//!
//! This is critical for CI/CD environments where browsers run in containers.

use playwright_rs::protocol::Playwright;
use playwright_rs::server::channel_owner::ChannelOwner;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

mod common;

/// Start a Playwright browser server using launchServer() and return the ws endpoint
///
/// This creates a Node.js script that:
/// 1. Requires Playwright
/// 2. Calls chromium.launchServer()
/// 3. Outputs the WebSocket endpoint to stdout
/// 4. Waits for stdin to close before shutting down
async fn start_browser_server(
    package_path: &std::path::Path,
) -> Option<(tokio::process::Child, String)> {
    // Node.js script to launch browser server
    let script = format!(
        r#"
const {{ chromium }} = require('{}');

(async () => {{
    const server = await chromium.launchServer({{ headless: true }});
    console.log(server.wsEndpoint());

    // Keep running until stdin closes (parent process exits)
    process.stdin.resume();
    process.stdin.on('close', async () => {{
        await server.close();
        process.exit(0);
    }});
}})().catch(err => {{
    console.error(err);
    process.exit(1);
}});
"#,
        package_path.display()
    );

    let mut child = Command::new("node")
        .arg("-e")
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    let stdout = child.stdout.take()?;
    let mut reader = BufReader::new(stdout).lines();

    // Wait for the WebSocket endpoint with timeout
    let ws_endpoint = tokio::time::timeout(Duration::from_secs(30), async {
        if let Ok(Some(line)) = reader.next_line().await {
            if line.starts_with("ws://") {
                return Some(line);
            }
        }
        None
    })
    .await
    .ok()??;

    Some((child, ws_endpoint))
}

/// Test connecting to a real Playwright browser server
///
/// This test:
/// 1. Launches a Playwright browser server via launchServer()
/// 2. Launches a local Playwright instance
/// 3. Connects to the browser server via WebSocket
/// 4. Performs browser operations on the remote browser
/// 5. Verifies everything works end-to-end
#[tokio::test]
async fn test_connect_to_real_server() {
    common::init_tracing();

    // Find the Playwright package path
    let drivers_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("drivers");

    let package_path = std::fs::read_dir(&drivers_dir)
        .ok()
        .and_then(|mut entries| entries.next())
        .and_then(|e| e.ok())
        .map(|e| e.path().join("package"));

    let package_path = match package_path {
        Some(p) if p.exists() => p,
        _ => {
            tracing::warn!(
                "Skipping test: Playwright driver not found in {:?}",
                drivers_dir
            );
            return;
        }
    };

    tracing::info!("Starting browser server");

    // Start the browser server
    let (mut server_process, ws_endpoint) = match start_browser_server(&package_path).await {
        Some(result) => result,
        None => {
            tracing::warn!("Skipping test: Failed to start browser server");
            return;
        }
    };

    tracing::info!("Browser server ready at {}", ws_endpoint);

    // Launch local Playwright to get access to BrowserType
    let playwright = match Playwright::launch().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Skipping test: Failed to launch local Playwright: {}", e);
            let _ = server_process.kill().await;
            return;
        }
    };

    tracing::info!("Connecting to remote server at {}", ws_endpoint);

    // Connect to the remote server
    let browser = match playwright.chromium().connect(&ws_endpoint, None).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to connect to remote server: {}", e);
            let _ = playwright.shutdown().await;
            let _ = server_process.kill().await;
            panic!("Connect failed: {:?}", e);
        }
    };

    tracing::info!("Connected! Browser GUID: {}", browser.guid());

    // Verify browser is connected
    assert!(browser.is_connected());
    assert!(!browser.version().is_empty());

    tracing::info!("Browser version: {}", browser.version());

    // Create a context and page on the remote browser
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // Navigate to a simple page
    page.goto("data:text/html,<h1>Hello from Remote!</h1>", None)
        .await
        .expect("Failed to navigate");

    // Verify the page loaded by checking content
    let locator = page.locator("h1").await;
    let content = locator.text_content().await.expect("Failed to get text");
    assert_eq!(
        content,
        Some("Hello from Remote!".to_string()),
        "Page content should match"
    );

    tracing::info!("✓ Remote connection test passed!");

    // Cleanup
    page.close().await.ok();
    context.close().await.ok();
    browser.close().await.ok();
    playwright.shutdown().await.ok();

    // Kill the server process (closing stdin triggers shutdown)
    let _ = server_process.kill().await;
}

/// Test connection timeout when server is not available
#[tokio::test]
async fn test_connect_timeout_when_server_unavailable() {
    common::init_tracing();

    // Launch local Playwright
    let playwright = match Playwright::launch().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Skipping test: Failed to launch Playwright: {}", e);
            return;
        }
    };

    // Try to connect to a port with no server (should fail quickly)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener); // Release the port
    let ws_endpoint = format!("ws://127.0.0.1:{}", port);

    tracing::info!(
        "Attempting to connect to unavailable server at {}",
        ws_endpoint
    );

    let start = std::time::Instant::now();

    // Use a short timeout for the test
    let options = playwright_rs::api::ConnectOptions::new().timeout(2000.0); // 2 seconds

    let result = playwright
        .chromium()
        .connect(&ws_endpoint, Some(options))
        .await;

    let elapsed = start.elapsed();

    // Should fail (connection refused or timeout)
    assert!(result.is_err(), "Expected connection to fail");

    // Should fail reasonably quickly (within timeout + some margin)
    assert!(
        elapsed < Duration::from_secs(5),
        "Connection took too long to fail: {:?}",
        elapsed
    );

    tracing::info!(
        "✓ Connection failed as expected in {:?}: {:?}",
        elapsed,
        result.unwrap_err()
    );

    playwright.shutdown().await.ok();
}

/// Test connecting with custom headers
#[tokio::test]
async fn test_connect_with_custom_headers() {
    common::init_tracing();

    // This test verifies that custom headers are passed through.
    // In a real scenario, this would be used for authentication tokens.
    //
    // For now, we just verify that passing headers doesn't break the connection.
    // A full test would require a server that validates headers.

    let drivers_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("drivers");

    let package_path = std::fs::read_dir(&drivers_dir)
        .ok()
        .and_then(|mut entries| entries.next())
        .and_then(|e| e.ok())
        .map(|e| e.path().join("package"));

    let package_path = match package_path {
        Some(p) if p.exists() => p,
        _ => {
            tracing::warn!("Skipping test: Playwright driver not found");
            return;
        }
    };

    // Start browser server
    let (mut server_process, ws_endpoint) = match start_browser_server(&package_path).await {
        Some(result) => result,
        None => {
            tracing::warn!("Skipping test: Failed to start browser server");
            return;
        }
    };

    let playwright = match Playwright::launch().await {
        Ok(p) => p,
        Err(e) => {
            let _ = server_process.kill().await;
            tracing::warn!("Skipping test: {}", e);
            return;
        }
    };

    // Connect with custom headers
    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Custom-Auth".to_string(), "test-token".to_string());
    headers.insert("X-Request-Id".to_string(), "12345".to_string());

    let options = playwright_rs::api::ConnectOptions::new()
        .headers(headers)
        .timeout(10000.0);

    let browser = match playwright
        .chromium()
        .connect(&ws_endpoint, Some(options))
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to connect: {}", e);
            let _ = playwright.shutdown().await;
            let _ = server_process.kill().await;
            panic!("Connect with headers failed: {:?}", e);
        }
    };

    assert!(browser.is_connected());
    tracing::info!("✓ Connected with custom headers successfully");

    // Cleanup
    browser.close().await.ok();
    playwright.shutdown().await.ok();
    let _ = server_process.kill().await;
}
