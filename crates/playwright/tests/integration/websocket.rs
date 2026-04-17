use crate::common;
use crate::test_server::TestServer;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_websocket_url() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let captured_url: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let captured_url_clone = captured_url.clone();

    page.on_websocket(move |ws| {
        let url_store = captured_url_clone.clone();
        Box::pin(async move {
            *url_store.lock().unwrap() = Some(ws.url().to_string());
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

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

    page.on_websocket(move |ws| {
        let ws_store = captured_ws_clone.clone();
        Box::pin(async move {
            *ws_store.lock().unwrap() = Some(ws);
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let ws = captured_ws.lock().unwrap().clone();
    assert!(ws.is_some(), "WebSocket should have been captured");
    let ws = ws.unwrap();

    assert!(!ws.is_closed(), "WebSocket should not be closed initially");

    page.close().await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    assert!(
        ws.is_closed(),
        "WebSocket should be closed after page close"
    );

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
async fn test_websocket_frame_received() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let received_frames: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let frames_clone = received_frames.clone();

    page.on_websocket(move |ws| {
        let frames_store = frames_clone.clone();
        Box::pin(async move {
            ws.on_frame_received(move |payload| {
                let store = frames_store.clone();
                Box::pin(async move {
                    store.lock().unwrap().push(payload);
                    Ok(())
                })
            })
            .await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    // The page sends "Hello Server" on open; the server echoes it back
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let frames = received_frames.lock().unwrap().clone();
    assert!(
        !frames.is_empty(),
        "Should have received at least one frame (echo from server)"
    );

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

    page.on_websocket(move |ws| {
        let ws_store = captured_ws_clone.clone();
        Box::pin(async move {
            *ws_store.lock().unwrap() = Some(ws);
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

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

    page.on_websocket(move |ws| {
        let ws_store = captured_ws_clone.clone();
        Box::pin(async move {
            *ws_store.lock().unwrap() = Some(ws);
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

    // Brief pause to let the WebSocket object be created and captured
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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

    let ws_url = server.url().replace("http://", "ws://") + "/ws";

    page.route_web_socket(&ws_url, move |route| {
        let called = handler_called_clone.clone();
        Box::pin(async move {
            called.store(true, Ordering::Release);
            route.connect_to_server().await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

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

    let ws_url = server.url().replace("http://", "ws://") + "/ws";

    context
        .route_web_socket(&ws_url, move |route| {
            let called = handler_called_clone.clone();
            Box::pin(async move {
                called.store(true, Ordering::Release);
                route.connect_to_server().await?;
                Ok(())
            })
        })
        .await
        .unwrap();

    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

    assert!(
        handler_called.load(Ordering::Acquire),
        "Context WebSocket route handler should have been called"
    );

    context.close().await.unwrap();
    browser.close().await.unwrap();
    server.shutdown();
}
