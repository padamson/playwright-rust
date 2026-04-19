//! WebSocket protocol object — represents a WebSocket connection in the page.
//!
//! # Example
//!
//! ```ignore
//! use playwright_rs::protocol::Playwright;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!
//!     // Set up the waiter BEFORE the action that opens the WebSocket
//!     let ws_waiter = page.expect_websocket(None).await?;
//!
//!     // Navigate to a page that opens a WebSocket
//!     page.goto("https://example.com/ws-demo", None).await?;
//!
//!     let ws = ws_waiter.wait().await?;
//!     println!("WebSocket URL: {}", ws.url());
//!
//!     // Wait for the connection to close
//!     let close_waiter = ws.expect_close(Some(5000.0)).await?;
//!     // ... trigger close ...
//!     close_waiter.wait().await?;
//!
//!     assert!(ws.is_closed());
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```

use crate::error::Result;
use crate::protocol::EventWaiter;
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde_json::Value;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// Represents a WebSocket connection initiated by a page.
///
/// `WebSocket` objects are created by the Playwright server when the page
/// opens a WebSocket connection. Use [`crate::protocol::Page::on_websocket`] to receive
/// `WebSocket` objects.
///
/// See: <https://playwright.dev/docs/api/class-websocket>
#[derive(Clone)]
pub struct WebSocket {
    base: ChannelOwnerImpl,
    /// The URL of the WebSocket connection.
    url: String,
    /// Tracks whether the WebSocket has been closed.
    is_closed: Arc<AtomicBool>,
    /// General event handlers (frameSent, frameReceived, socketError, close).
    handlers: Arc<Mutex<Vec<WebSocketEventHandler>>>,
    /// One-shot senders waiting for the next "close" event.
    close_waiters: Arc<Mutex<Vec<oneshot::Sender<()>>>>,
    /// One-shot senders waiting for the next "frameReceived" event.
    frame_received_waiters: Arc<Mutex<Vec<oneshot::Sender<String>>>>,
    /// One-shot senders waiting for the next "frameSent" event.
    frame_sent_waiters: Arc<Mutex<Vec<oneshot::Sender<String>>>>,
}

/// Type alias for boxed event handler future.
type WebSocketEventHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// WebSocket event handler type.
type WebSocketEventHandler =
    Arc<dyn Fn(WebSocketEvent) -> WebSocketEventHandlerFuture + Send + Sync>;

#[derive(Clone, Debug)]
enum WebSocketEvent {
    FrameSent(String),
    FrameReceived(String),
    SocketError(String),
    Close,
}

impl WebSocket {
    /// Creates a new `WebSocket` object.
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        let url = initializer["url"].as_str().unwrap_or("").to_string();

        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer,
        );

        Ok(Self {
            base,
            url,
            is_closed: Arc::new(AtomicBool::new(false)),
            handlers: Arc::new(Mutex::new(Vec::new())),
            close_waiters: Arc::new(Mutex::new(Vec::new())),
            frame_received_waiters: Arc::new(Mutex::new(Vec::new())),
            frame_sent_waiters: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Returns the URL of the WebSocket connection.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocket#web-socket-url>
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns `true` if the WebSocket is closed.
    ///
    /// The value becomes `true` when the `"close"` event fires (i.e. when the
    /// underlying TCP connection is torn down). It remains `false` from
    /// construction until that point.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocket#web-socket-is-closed>
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Acquire)
    }

    /// Registers a handler that fires when a frame is sent from the page to the server.
    ///
    /// The handler receives the frame payload as a `String`. For binary frames the
    /// value is the base-64-encoded representation.
    ///
    /// # Errors
    ///
    /// Returns an error only if the handler cannot be registered (in practice
    /// this never fails).
    ///
    /// See: <https://playwright.dev/docs/api/class-websocket#web-socket-event-frame-sent>
    pub async fn on_frame_sent<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(String) -> WebSocketEventHandlerFuture + Send + Sync + 'static,
    {
        let handler_arc = Arc::new(move |event| match event {
            WebSocketEvent::FrameSent(payload) => handler(payload),
            _ => Box::pin(async { Ok(()) }),
        });
        self.handlers.lock().unwrap().push(handler_arc);
        Ok(())
    }

    /// Registers a handler that fires when a frame is received from the server.
    ///
    /// The handler receives the frame payload as a `String`. For binary frames the
    /// value is the base-64-encoded representation.
    ///
    /// # Errors
    ///
    /// Returns an error only if the handler cannot be registered (in practice
    /// this never fails).
    ///
    /// See: <https://playwright.dev/docs/api/class-websocket#web-socket-event-frame-received>
    pub async fn on_frame_received<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(String) -> WebSocketEventHandlerFuture + Send + Sync + 'static,
    {
        let handler_arc = Arc::new(move |event| match event {
            WebSocketEvent::FrameReceived(payload) => handler(payload),
            _ => Box::pin(async { Ok(()) }),
        });
        self.handlers.lock().unwrap().push(handler_arc);
        Ok(())
    }

    /// Registers a handler that fires when the WebSocket encounters an error.
    ///
    /// The handler receives the error message as a `String`.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocket#web-socket-event-socket-error>
    pub async fn on_error<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(String) -> WebSocketEventHandlerFuture + Send + Sync + 'static,
    {
        let handler_arc = Arc::new(move |event| match event {
            WebSocketEvent::SocketError(msg) => handler(msg),
            _ => Box::pin(async { Ok(()) }),
        });
        self.handlers.lock().unwrap().push(handler_arc);
        Ok(())
    }

    /// Registers a handler that fires when the WebSocket is closed.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocket#web-socket-event-close>
    pub async fn on_close<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(()) -> WebSocketEventHandlerFuture + Send + Sync + 'static,
    {
        let handler_arc = Arc::new(move |event| match event {
            WebSocketEvent::Close => handler(()),
            _ => Box::pin(async { Ok(()) }),
        });
        self.handlers.lock().unwrap().push(handler_arc);
        Ok(())
    }

    /// Creates a one-shot waiter that resolves when the WebSocket is closed.
    ///
    /// The waiter **must** be created before the action that closes the
    /// WebSocket to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` — Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Timeout`](crate::error::Error::Timeout) if the WebSocket
    /// is not closed within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocket#web-socket-wait-for-event>
    pub async fn expect_close(&self, timeout: Option<f64>) -> Result<EventWaiter<()>> {
        let (tx, rx) = oneshot::channel();
        self.close_waiters.lock().unwrap().push(tx);
        Ok(EventWaiter::new(rx, timeout.or(Some(30_000.0))))
    }

    /// Creates a one-shot waiter that resolves when the next frame is received from the server.
    ///
    /// The waiter **must** be created before the action that causes a frame to be
    /// received to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` — Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Timeout`](crate::error::Error::Timeout) if no frame is
    /// received within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocket#web-socket-wait-for-event>
    pub async fn expect_frame_received(&self, timeout: Option<f64>) -> Result<EventWaiter<String>> {
        let (tx, rx) = oneshot::channel();
        self.frame_received_waiters.lock().unwrap().push(tx);
        Ok(EventWaiter::new(rx, timeout.or(Some(30_000.0))))
    }

    /// Creates a one-shot waiter that resolves when the next frame is sent from the page.
    ///
    /// The waiter **must** be created before the action that sends the frame to
    /// avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` — Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Timeout`](crate::error::Error::Timeout) if no frame is
    /// sent within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocket#web-socket-wait-for-event>
    pub async fn expect_frame_sent(&self, timeout: Option<f64>) -> Result<EventWaiter<String>> {
        let (tx, rx) = oneshot::channel();
        self.frame_sent_waiters.lock().unwrap().push(tx);
        Ok(EventWaiter::new(rx, timeout.or(Some(30_000.0))))
    }

    /// Dispatches a server-sent event to all registered handlers and waiters.
    pub(crate) fn handle_event(&self, event: &str, params: &Value) {
        let ws_event = match event {
            "frameSent" => {
                WebSocketEvent::FrameSent(params["data"].as_str().unwrap_or("").to_string())
            }
            "frameReceived" => {
                WebSocketEvent::FrameReceived(params["data"].as_str().unwrap_or("").to_string())
            }
            "socketError" => {
                WebSocketEvent::SocketError(params["error"].as_str().unwrap_or("").to_string())
            }
            "close" => {
                // Mark as closed before notifying waiters so is_closed() is true
                // when any await continuation runs.
                self.is_closed.store(true, Ordering::Release);
                WebSocketEvent::Close
            }
            _ => return,
        };

        // Notify one-shot waiters for specific event types
        match &ws_event {
            WebSocketEvent::Close => {
                let waiters: Vec<_> = std::mem::take(&mut *self.close_waiters.lock().unwrap());
                for tx in waiters {
                    let _ = tx.send(());
                }
            }
            WebSocketEvent::FrameReceived(payload) => {
                if let Some(tx) = self.frame_received_waiters.lock().unwrap().pop() {
                    let _ = tx.send(payload.clone());
                }
            }
            WebSocketEvent::FrameSent(payload) => {
                if let Some(tx) = self.frame_sent_waiters.lock().unwrap().pop() {
                    let _ = tx.send(payload.clone());
                }
            }
            WebSocketEvent::SocketError(_) => {}
        }

        // Notify general handlers (fire-and-forget)
        let handlers = self.handlers.lock().unwrap().clone();
        for handler in handlers {
            let event = ws_event.clone();
            tokio::spawn(async move {
                let _ = handler(event).await;
            });
        }
    }
}

impl ChannelOwner for WebSocket {
    fn guid(&self) -> &str {
        self.base.guid()
    }

    fn type_name(&self) -> &str {
        self.base.type_name()
    }

    fn parent(&self) -> Option<Arc<dyn ChannelOwner>> {
        self.base.parent()
    }

    fn connection(&self) -> Arc<dyn crate::server::connection::ConnectionLike> {
        self.base.connection()
    }

    fn initializer(&self) -> &Value {
        self.base.initializer()
    }

    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    fn dispose(&self, reason: crate::server::channel_owner::DisposeReason) {
        // When the WebSocket object is disposed (page closed, or server-initiated),
        // mark it as closed and satisfy any pending close waiters — even if the
        // "close" event was never explicitly delivered before __dispose__.
        self.is_closed.store(true, Ordering::Release);
        let waiters: Vec<_> = std::mem::take(&mut *self.close_waiters.lock().unwrap());
        for tx in waiters {
            let _ = tx.send(());
        }
        self.base.dispose(reason)
    }

    fn adopt(&self, child: Arc<dyn ChannelOwner>) {
        self.base.adopt(child)
    }

    fn add_child(&self, guid: Arc<str>, child: Arc<dyn ChannelOwner>) {
        self.base.add_child(guid, child)
    }

    fn remove_child(&self, guid: &str) {
        self.base.remove_child(guid)
    }

    fn on_event(&self, method: &str, params: Value) {
        self.handle_event(method, &params);
        self.base.on_event(method, params)
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
