use futures_util::{SinkExt, StreamExt};
use playwright_rs::protocol::Playwright;
use playwright_rs::server::channel_owner::ChannelOwner;
use playwright_rs::server::connection::Connection;
use playwright_rs::server::transport::PipeTransport;
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

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
