use futures_util::{SinkExt, StreamExt};
use playwright_rs::server::transport::WebSocketTransport;
use playwright_rs::server::transport::{TransportReceiver, TransportSender};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

#[tokio::test]
async fn test_websocket_transport_communication() {
    // 1. Start a mock WebSocket server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws_stream = accept_async(stream).await.unwrap();

        // Echo server: receive message, send it back
        while let Some(msg) = ws_stream.next().await {
            let msg = msg.unwrap();
            if msg.is_text() || msg.is_binary() {
                ws_stream.send(msg).await.unwrap();
            }
        }
    });

    // 2. Connect using WebSocketTransport
    let url = format!("ws://{}", addr);
    let (transport, mut message_rx) = WebSocketTransport::connect(&url, None).await.unwrap();

    // 3. Split transport (simulating Connection usage)
    let (mut sender, mut receiver) = transport.into_parts();

    // 4. Spawn receiver loop
    let receiver_task = tokio::spawn(async move {
        receiver.run().await.unwrap();
    });

    // 5. Send a message
    let test_message = json!({
        "id": 1,
        "method": "echo",
        "params": {
            "hello": "world"
        }
    });

    sender.send(test_message.clone()).await.unwrap();

    // 6. Verify we receive it back (echo)
    let received = message_rx.recv().await.unwrap();
    assert_eq!(received, test_message);

    // Cleanup
    server_task.abort();
    receiver_task.abort();
}
