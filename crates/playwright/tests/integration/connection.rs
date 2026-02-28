// Integration tests for Connection layer with real Playwright server
//
// These tests verify that the Connection layer can:
// - Establish connection to real Playwright server
// - Spawn transport and connection message loops
// - Handle protocol initialization messages from server
//
// Note: Full protocol request/response testing will be implemented in Slice 4
// (Object Factory) when we can handle the initialization sequence and send requests.

use futures_util::{SinkExt, StreamExt};
use playwright_rs::protocol::Playwright;
use playwright_rs::server::channel_owner::ChannelOwner;
use playwright_rs::server::connection::Connection;
use playwright_rs::server::playwright_server::PlaywrightServer;
use playwright_rs::server::transport::PipeTransport;
use serde_json::json;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Test that we can establish a connection with real server and spawn message loops
///
/// This test verifies:
/// - Server launches successfully
/// - Connection can be created with server stdio
/// - Message loops can be spawned without errors
/// - Everything runs together and shuts down cleanly
#[tokio::test]
async fn test_connection_lifecycle_with_real_server() {
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
    crate::common::init_tracing();
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
    crate::common::init_tracing();
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

// ============================================================================
// Merged from: connect_over_cdp_test.rs
// ============================================================================

// Integration tests for BrowserType::connect_over_cdp()
//
// Tests cover:
// - Chromium-only enforcement (Firefox/WebKit should fail)
// - Real CDP connection to a Chrome instance with remote debugging

/// Test that connect_over_cdp fails for Firefox (Chromium-only)
#[tokio::test]
async fn test_connect_over_cdp_chromium_only() {
    crate::common::init_tracing();

    let playwright = match Playwright::launch().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Skipping test: Failed to launch Playwright: {}", e);
            return;
        }
    };

    // Firefox should fail
    let result = playwright
        .firefox()
        .connect_over_cdp("http://localhost:9222", None)
        .await;
    assert!(
        result.is_err(),
        "Firefox should not support connect_over_cdp"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Chromium"),
        "Error should mention Chromium: {}",
        err
    );

    // WebKit should fail
    let result = playwright
        .webkit()
        .connect_over_cdp("http://localhost:9222", None)
        .await;
    assert!(
        result.is_err(),
        "WebKit should not support connect_over_cdp"
    );

    playwright.shutdown().await.ok();
}

/// Launch Chrome with --remote-debugging-port and return the CDP endpoint URL.
///
/// Uses Playwright Node.js to find the Chrome binary and launch it with
/// remote debugging enabled, then discovers the CDP endpoint via /json/version.
async fn start_chrome_with_cdp(
    package_path: &std::path::Path,
) -> Option<(tokio::process::Child, String)> {
    // Node.js script that:
    // 1. Gets Chrome executable path from Playwright
    // 2. Spawns Chrome with --remote-debugging-port=0
    // 3. Reads the DevTools URL from stderr
    // 4. Outputs the HTTP endpoint to stdout
    let script = format!(
        r#"
const {{ chromium }} = require('{}');
const {{ spawn }} = require('child_process');
const http = require('http');

const execPath = chromium.executablePath();

const child = spawn(execPath, [
    '--headless',
    '--remote-debugging-port=0',
    '--no-sandbox',
    '--disable-gpu',
    '--use-mock-keychain',
    '--no-first-run'
], {{ stdio: ['pipe', 'pipe', 'pipe'] }});

// Chrome outputs the DevTools URL to stderr
let stderr = '';
child.stderr.on('data', (data) => {{
    stderr += data.toString();
    // Look for the DevTools listening message
    const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
    if (match) {{
        // Extract port from ws://127.0.0.1:PORT/devtools/browser/...
        const portMatch = match[1].match(/:(\d+)\//);
        if (portMatch) {{
            console.log('http://127.0.0.1:' + portMatch[1]);
        }}
    }}
}});

// Keep running until stdin closes
process.stdin.resume();
process.stdin.on('close', () => {{
    child.kill();
    process.exit(0);
}});

// Also kill on timeout (safety)
setTimeout(() => {{
    child.kill();
    process.exit(1);
}}, 25000);
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
    let mut reader = tokio::io::BufReader::new(stdout).lines();

    // Wait for the CDP endpoint with timeout
    let endpoint = tokio::time::timeout(Duration::from_secs(15), async {
        while let Ok(Some(line)) = reader.next_line().await {
            if line.starts_with("http://") || line.starts_with("ws://") {
                return Some(line);
            }
        }
        None
    })
    .await
    .ok()??;

    Some((child, endpoint))
}

/// Test connecting to a real Chrome via CDP
#[tokio::test]
async fn test_connect_over_cdp_real_chrome() {
    crate::common::init_tracing();

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
            tracing::warn!("Skipping test: Playwright driver not found");
            return;
        }
    };

    // Start Chrome with CDP
    let (mut chrome_process, cdp_endpoint) = match start_chrome_with_cdp(&package_path).await {
        Some(result) => result,
        None => {
            tracing::warn!("Skipping test: Failed to start Chrome with CDP");
            return;
        }
    };

    tracing::info!("Chrome CDP endpoint: {}", cdp_endpoint);

    // Launch local Playwright
    let playwright = match Playwright::launch().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Skipping test: Failed to launch Playwright: {}", e);
            let _ = chrome_process.kill().await;
            return;
        }
    };

    // Connect over CDP
    let browser = match playwright
        .chromium()
        .connect_over_cdp(&cdp_endpoint, None)
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to connect over CDP: {}", e);
            let _ = playwright.shutdown().await;
            let _ = chrome_process.kill().await;
            panic!("connect_over_cdp failed: {:?}", e);
        }
    };

    tracing::info!("Connected via CDP! Browser version: {}", browser.version());

    // Verify browser works
    assert!(browser.is_connected());
    assert!(!browser.version().is_empty());

    // Create a page and navigate
    let page = browser.new_page().await.expect("Failed to create page");
    page.goto("data:text/html,<h1>CDP Connection Works!</h1>", None)
        .await
        .expect("Failed to navigate");

    let heading = page.locator("h1").await;
    let text = heading.text_content().await.expect("Failed to get text");
    assert_eq!(text, Some("CDP Connection Works!".to_string()));

    tracing::info!("CDP connection test passed!");

    // Cleanup
    browser.close().await.ok();
    playwright.shutdown().await.ok();
    let _ = chrome_process.kill().await;
}

// ============================================================================
// Merged from: browser_type_connect_test.rs
// ============================================================================

#[tokio::test]
async fn test_browser_type_connect() {
    eprintln!("Test starting");

    // 1. Setup Remote Mock Server (TCP)
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    eprintln!("Remote Mock server bound");
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://127.0.0.1:{}", addr.port());

    // Spawn Remote Mock Server logic
    tokio::spawn(async move {
        // Accept incoming connection
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws_stream = accept_async(stream).await.unwrap();

        let browser_guid = "browser@remote"; // Remote browser GUID

        // Send objects for Remote Connection
        // Order: BrowserTypes (Root) -> Browser (Child) -> Playwright (Root, referencing others)

        let types = vec!["chromium", "firefox", "webkit"];
        for t in types {
            let create_type = json!({
                "guid": "", // Root
                "method": "__create__",
                "params": {
                    "type": "BrowserType",
                    "guid": format!("browserType@{}", t),
                    "initializer": {
                        "name": t,
                        "executablePath": "/bin/browser"
                    }
                }
            });
            ws_stream
                .send(Message::Text(create_type.to_string().into()))
                .await
                .unwrap();
        }

        let create_browser = json!({
            "guid": "browserType@chromium",
            "method": "__create__",
            "params": {
                "type": "Browser",
                "guid": browser_guid,
                "initializer": {
                    "name": "chromium",
                    "executablePath": "/bin/chromium",
                    "version": "1.0"
                }
            }
        });
        ws_stream
            .send(Message::Text(create_browser.to_string().into()))
            .await
            .unwrap();

        let create_playwright = json!({
            "guid": "", // Root
            "method": "__create__",
            "params": {
                "type": "Playwright",
                "guid": "playwright",
                "initializer": {
                    "chromium": { "guid": "browserType@chromium" },
                    "firefox": { "guid": "browserType@firefox" },
                    "webkit": { "guid": "browserType@webkit" },
                    "preLaunchedBrowser": { "guid": browser_guid }
                }
            }
        });
        ws_stream
            .send(Message::Text(create_playwright.to_string().into()))
            .await
            .unwrap();

        // Handle incoming messages (especially the initialize request)
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(request) = serde_json::from_str::<serde_json::Value>(&text) {
                        // Check if this is an initialize request
                        if request.get("method").and_then(|m| m.as_str()) == Some("initialize") {
                            let id = request.get("id").and_then(|i| i.as_u64()).unwrap_or(0);
                            let response = json!({
                                "id": id,
                                "result": {
                                    "playwright": {
                                        "guid": "playwright"
                                    }
                                }
                            });
                            ws_stream
                                .send(Message::Text(response.to_string().into()))
                                .await
                                .unwrap();
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    // 2. Setup Local Mock Connection (Duplex) to simulate local driver
    let (client_conn, mut server_conn) = tokio::io::duplex(65536);

    // Spawn Local Mock Server
    tokio::spawn(async move {
        // Helper to send framed message
        async fn send_framed(stream: &mut tokio::io::DuplexStream, msg: serde_json::Value) {
            let bytes = serde_json::to_vec(&msg).unwrap();
            let len = bytes.len() as u32;
            stream.write_all(&len.to_le_bytes()).await.unwrap();
            stream.write_all(&bytes).await.unwrap();
        }

        // Send BrowserTypes (Roots)
        let types = vec!["chromium", "firefox", "webkit"];
        for t in types {
            let create_type = json!({
                "guid": "",
                "method": "__create__",
                "params": {
                    "type": "BrowserType",
                    "guid": format!("browserType@{}", t),
                    "initializer": {
                        "name": t,
                        "executablePath": "/bin/browser"
                    }
                }
            });
            send_framed(&mut server_conn, create_type).await;
        }

        // Send Playwright (Root). No preLaunchedBrowser locally usually.
        let create_playwright = json!({
            "guid": "",
            "method": "__create__",
            "params": {
                "type": "Playwright",
                "guid": "playwright",
                "initializer": {
                    "chromium": { "guid": "browserType@chromium" },
                    "firefox": { "guid": "browserType@firefox" },
                    "webkit": { "guid": "browserType@webkit" }
                }
            }
        });
        send_framed(&mut server_conn, create_playwright).await;

        // Read "initialize" request
        let mut len_buf = [0u8; 4];
        server_conn.read_exact(&mut len_buf).await.unwrap();
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut msg_buf = vec![0u8; len];
        server_conn.read_exact(&mut msg_buf).await.unwrap();

        let msg: serde_json::Value = serde_json::from_slice(&msg_buf).unwrap();
        if let Some(id) = msg["id"].as_i64() {
            let response = json!({
                "id": id,
                "result": {
                    "playwright": {
                        "guid": "playwright" // Must match the GUID sent in __create__
                    }
                }
            });
            send_framed(&mut server_conn, response).await;
        }

        // Consume further input (keep connection open)
        let mut buf = vec![0u8; 1024];
        loop {
            if server_conn.read(&mut buf).await.unwrap() == 0 {
                break;
            }
        }
    });

    // 3. Initialize Client (Local)
    let (client_r, client_w) = tokio::io::split(client_conn);
    let (transport, message_rx) = PipeTransport::new(client_w, client_r);
    let (sender, receiver) = transport.into_parts();

    let connection = Arc::new(Connection::new(sender, receiver, message_rx));
    let conn_clone = connection.clone();
    tokio::spawn(async move {
        conn_clone.run().await;
    });

    // Give the connection a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    eprintln!("Initializing local playwright");
    let playwright_obj = connection
        .initialize_playwright()
        .await
        .expect("Local init failed");
    let playwright = playwright_obj
        .as_any()
        .downcast_ref::<Playwright>()
        .unwrap();
    eprintln!("Local playwright initialized");

    // 4. Connect to Remote
    eprintln!("Connecting to remote: {}", url);
    let browser = playwright
        .chromium()
        .connect(&url, None)
        .await
        .expect("Connect failed");
    eprintln!("Connected!");

    assert_eq!(browser.guid(), "browser@remote");
}

// ============================================================================
// Merged from: remote_connection_test.rs
// ============================================================================

// Integration tests for remote browser connection via WebSocket
//
// These tests verify that `BrowserType::connect()` works with a real
// Playwright browser server created via `chromium.launchServer()`.
//
// This is critical for CI/CD environments where browsers run in containers.

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
    crate::common::init_tracing();

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
    crate::common::init_tracing();

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
    crate::common::init_tracing();

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
