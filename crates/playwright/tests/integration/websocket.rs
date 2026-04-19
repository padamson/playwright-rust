use crate::common;
use crate::test_server::TestServer;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

#[tokio::test]
async fn test_websocket_url() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let captured_url: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let captured_url_clone = captured_url.clone();
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    page.on_websocket(move |ws| {
        let url_store = captured_url_clone.clone();
        let n = notify_clone.clone();
        Box::pin(async move {
            *url_store.lock().unwrap() = Some(ws.url().to_string());
            n.notify_one();
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::timeout(std::time::Duration::from_secs(5), notify.notified())
        .await
        .expect("websocket event did not fire");

    let url = captured_url.lock().unwrap().clone();
    assert!(url.is_some(), "WebSocket URL should have been captured");
    let url = url.unwrap();
    assert!(url.contains("/ws"), "URL should contain /ws, got: {url}");

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
async fn test_websocket_is_closed() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let captured_ws: Arc<Mutex<Option<playwright_rs::protocol::WebSocket>>> =
        Arc::new(Mutex::new(None));
    let captured_ws_clone = captured_ws.clone();
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    page.on_websocket(move |ws| {
        let ws_store = captured_ws_clone.clone();
        let n = notify_clone.clone();
        Box::pin(async move {
            *ws_store.lock().unwrap() = Some(ws);
            n.notify_one();
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::timeout(std::time::Duration::from_secs(5), notify.notified())
        .await
        .expect("websocket event did not fire");

    let ws = captured_ws.lock().unwrap().clone();
    assert!(ws.is_some(), "WebSocket should have been captured");
    let ws = ws.unwrap();

    assert!(!ws.is_closed(), "WebSocket should not be closed initially");

    // Set up the close waiter BEFORE closing the page
    let close_waiter = ws.expect_close(Some(5000.0)).await.unwrap();

    page.close().await.unwrap();

    close_waiter
        .wait()
        .await
        .expect("expect_close waiter should resolve after page close");

    assert!(
        ws.is_closed(),
        "WebSocket should be closed after waiter resolves"
    );

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
#[ignore] // flaky under concurrency — passes in isolation, fails with parallel tests
async fn test_websocket_frame_received() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    // Channel to hand the frame waiter out of the on_websocket callback
    let (waiter_tx, waiter_rx) =
        tokio::sync::oneshot::channel::<playwright_rs::EventWaiter<String>>();
    let waiter_tx = Arc::new(Mutex::new(Some(waiter_tx)));

    page.on_websocket(move |ws| {
        let tx_store = waiter_tx.clone();
        Box::pin(async move {
            // Set up the waiter as soon as we have the ws object.
            // The echo frame may not have arrived yet at this point.
            let waiter = ws.expect_frame_received(Some(5000.0)).await?;
            if let Some(tx) = tx_store.lock().unwrap().take() {
                let _ = tx.send(waiter);
            }
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    // The page sends "Hello Server" on open; the server echoes it back.
    // Receive the waiter that was created inside on_websocket and wait for the frame.
    let waiter = tokio::time::timeout(std::time::Duration::from_secs(5), waiter_rx)
        .await
        .expect("on_websocket did not fire in time")
        .expect("waiter_tx was dropped without sending");

    let frame = waiter
        .wait()
        .await
        .expect("should have received at least one frame (echo from server)");

    assert!(!frame.is_empty(), "Received frame should not be empty");

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
async fn test_websocket_expect_close() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let captured_ws: Arc<Mutex<Option<playwright_rs::protocol::WebSocket>>> =
        Arc::new(Mutex::new(None));
    let captured_ws_clone = captured_ws.clone();
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    page.on_websocket(move |ws| {
        let ws_store = captured_ws_clone.clone();
        let n = notify_clone.clone();
        Box::pin(async move {
            *ws_store.lock().unwrap() = Some(ws);
            n.notify_one();
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::timeout(std::time::Duration::from_secs(5), notify.notified())
        .await
        .expect("websocket event did not fire");

    let ws = captured_ws.lock().unwrap().clone();
    assert!(ws.is_some(), "WebSocket should have been captured");
    let ws = ws.unwrap();

    // Set up the close waiter BEFORE closing
    let waiter = ws.expect_close(Some(5000.0)).await.unwrap();

    // Close the page to trigger WebSocket close
    page.close().await.unwrap();

    // The waiter should resolve
    waiter
        .wait()
        .await
        .expect("expect_close waiter should resolve after page close");

    assert!(
        ws.is_closed(),
        "WebSocket should be closed after waiter resolves"
    );

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
#[ignore] // flaky under concurrency — passes in isolation, fails with parallel tests
async fn test_websocket_expect_frame_received_api() {
    // Verifies that expect_frame_received compiles and returns an EventWaiter<String>
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let captured_ws: Arc<Mutex<Option<playwright_rs::protocol::WebSocket>>> =
        Arc::new(Mutex::new(None));
    let captured_ws_clone = captured_ws.clone();
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    page.on_websocket(move |ws| {
        let ws_store = captured_ws_clone.clone();
        let n = notify_clone.clone();
        Box::pin(async move {
            *ws_store.lock().unwrap() = Some(ws);
            n.notify_one();
            Ok(())
        })
    })
    .await
    .unwrap();

    // Set up the waiter BEFORE navigation to avoid missing the first frame
    // (the page sends "Hello Server" immediately on connect)
    // We can't do that here because we need the ws object first.
    // Instead navigate and capture the ws, then wait for any subsequent frame.
    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    // Wait for the WebSocket object to be created and captured
    tokio::time::timeout(std::time::Duration::from_secs(5), notify.notified())
        .await
        .expect("websocket event did not fire");

    let ws = captured_ws.lock().unwrap().clone();
    assert!(ws.is_some(), "WebSocket should have been captured");
    let ws = ws.unwrap();

    // Verify expect_frame_received creates an EventWaiter<String>
    let _waiter = ws
        .expect_frame_received(Some(2000.0))
        .await
        .expect("expect_frame_received should return Ok");

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
async fn test_page_route_web_socket() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    use std::sync::atomic::{AtomicBool, Ordering};
    let handler_called = Arc::new(AtomicBool::new(false));
    let handler_called_clone = handler_called.clone();
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    let ws_url = server.url().replace("http://", "ws://") + "/ws";

    page.route_web_socket(&ws_url, move |route| {
        let called = handler_called_clone.clone();
        let n = notify_clone.clone();
        Box::pin(async move {
            called.store(true, Ordering::Release);
            n.notify_one();
            route.connect_to_server().await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::timeout(std::time::Duration::from_secs(5), notify.notified())
        .await
        .expect("WebSocket route handler did not fire");

    assert!(
        handler_called.load(Ordering::Acquire),
        "WebSocket route handler should have been called"
    );

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
async fn test_context_route_web_socket() {
    let (_pw, browser, context) = common::setup_context().await;
    let server = TestServer::start().await;
    let page = context.new_page().await.unwrap();

    use std::sync::atomic::{AtomicBool, Ordering};
    let handler_called = Arc::new(AtomicBool::new(false));
    let handler_called_clone = handler_called.clone();
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    let ws_url = server.url().replace("http://", "ws://") + "/ws";

    context
        .route_web_socket(&ws_url, move |route| {
            let called = handler_called_clone.clone();
            let n = notify_clone.clone();
            Box::pin(async move {
                called.store(true, Ordering::Release);
                n.notify_one();
                route.connect_to_server().await?;
                Ok(())
            })
        })
        .await
        .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::timeout(std::time::Duration::from_secs(5), notify.notified())
        .await
        .expect("Context WebSocket route handler did not fire");

    assert!(
        handler_called.load(Ordering::Acquire),
        "Context WebSocket route handler should have been called"
    );

    context.close().await.unwrap();
    browser.close().await.unwrap();
    server.shutdown();
}
