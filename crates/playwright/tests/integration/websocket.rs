use crate::common;
use crate::test_server::TestServer;
use playwright_rs::expect;
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::test]
async fn test_websocket_url() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let capture = crate::common::capture_websocket(&page).await;
    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();
    let url = capture.wait().await.url().to_string();
    assert!(url.contains("/ws"), "URL should contain /ws, got: {url}");

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
async fn test_websocket_is_closed() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let capture = crate::common::capture_websocket(&page).await;
    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();
    let ws = capture.wait().await;

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

// The waiter is armed before the page is asked to send, and the page sends
// nothing on its own, so the only frame that can arrive is the one asked for.
// The old version raced the echo of a frame the page sent on open.
#[tokio::test]
async fn expect_frame_received_resolves_with_the_frame_the_server_sent() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let capture = crate::common::capture_websocket(&page).await;
    page.goto(&format!("{}/websocket_quiet.html", server.url()), None)
        .await
        .unwrap();
    let ws = capture.wait().await;

    let waiter = ws.expect_frame_received(Some(5000.0)).await.unwrap();
    page.evaluate_expression("send('ping from test')")
        .await
        .unwrap();

    let frame = waiter
        .wait()
        .await
        .expect("no frame received after the page sent one");
    assert_eq!(frame, "ping from test");

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
async fn test_websocket_expect_close() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;

    let capture = crate::common::capture_websocket(&page).await;
    page.goto(&format!("{}/websocket.html", server.url()), None)
        .await
        .unwrap();
    let ws = capture.wait().await;

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

    // connect_to_server proxied the page's socket to the real server, so the
    // server's echo reaches the page.
    expect(page.locator("#log"))
        .to_contain_text("received: Hello Server")
        .await
        .expect("echo from the real server reached the page");

    assert!(
        handler_called.load(Ordering::Acquire),
        "WebSocket route handler should have been called"
    );

    browser.close().await.unwrap();
    server.shutdown();
}

#[tokio::test]
async fn test_page_route_web_socket_mock_replies_without_a_server() {
    let (_pw, browser, page) = common::setup().await;
    let server = TestServer::start().await;
    let ws_url = server.url().replace("http://", "ws://") + "/ws";

    // No connect_to_server: the handler answers the page itself.
    page.route_web_socket(&ws_url, move |route| {
        Box::pin(async move {
            let replier = route.clone();
            route
                .on_message(move |message| {
                    let replier = replier.clone();
                    Box::pin(async move { replier.send(&format!("mock: {message}")).await })
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

    expect(page.locator("#log"))
        .to_contain_text("received: mock: Hello Server")
        .await
        .expect("the mocked reply reached the page");

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

    // connect_to_server proxied the page's socket to the real server, so the
    // server's echo reaches the page.
    expect(page.locator("#log"))
        .to_contain_text("received: Hello Server")
        .await
        .expect("echo from the real server reached the page");

    assert!(
        handler_called.load(Ordering::Acquire),
        "Context WebSocket route handler should have been called"
    );

    context.close().await.unwrap();
    browser.close().await.unwrap();
    server.shutdown();
}
