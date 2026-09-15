//! WebSocketRoute protocol object — represents an intercepted WebSocket connection.
//!
//! `WebSocketRoute` is created by the Playwright server when a WebSocket connection
//! matches a pattern registered via [`crate::protocol::Page::route_web_socket`] or
//! [`crate::protocol::BrowserContext::route_web_socket`].
//!
//! # Example
//!
//! ```no_run
//! use playwright_rs::protocol::Playwright;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!
//!     // Intercept all WebSocket connections and proxy them to the real server
//!     page.route_web_socket("ws://**", |route| {
//!         Box::pin(async move {
//!             route.connect_to_server().await?;
//!             Ok(())
//!         })
//!     })
//!     .await?;
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-websocketroute>

use crate::error::Result;
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde_json::Value;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Represents an intercepted WebSocket connection.
///
/// `WebSocketRoute` is passed to handlers registered via
/// [`crate::protocol::Page::route_web_socket`] or [`crate::protocol::BrowserContext::route_web_socket`].
/// The handler must call [`connect_to_server`](WebSocketRoute::connect_to_server)
/// to forward the connection to the real server, or [`close`](WebSocketRoute::close)
/// to terminate it.
///
/// See: <https://playwright.dev/docs/api/class-websocketroute>
#[derive(Clone)]
pub struct WebSocketRoute {
    base: ChannelOwnerImpl,
    /// The WebSocket URL being intercepted.
    url: String,
    /// Message handlers registered via on_message().
    message_handlers: Arc<Mutex<Vec<WebSocketRouteMessageHandler>>>,
    /// Close handlers registered via on_close().
    close_handlers: Arc<Mutex<Vec<WebSocketRouteCloseHandler>>>,
    /// Set by `connect_to_server`; decides whether page messages without a
    /// handler are forwarded to the real server or dropped.
    connected: Arc<AtomicBool>,
}

/// Type alias for boxed WebSocketRoute message handler future.
type WebSocketRouteHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Message handler type.
type WebSocketRouteMessageHandler =
    Arc<dyn Fn(String) -> WebSocketRouteHandlerFuture + Send + Sync>;

/// Close handler type.
type WebSocketRouteCloseHandler = Arc<dyn Fn() -> WebSocketRouteHandlerFuture + Send + Sync>;

impl WebSocketRoute {
    /// Creates a new `WebSocketRoute` object.
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
            message_handlers: Arc::new(Mutex::new(Vec::new())),
            close_handlers: Arc::new(Mutex::new(Vec::new())),
            connected: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Returns the URL of the intercepted WebSocket connection.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocketroute#web-socket-route-url>
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the WebSocket subprotocols the page requested (the
    /// `Sec-WebSocket-Protocol` values) when opening this socket. Empty if none
    /// were requested.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocketroute#web-socket-route-protocols>
    pub fn protocols(&self) -> Vec<String> {
        self.base
            .initializer()
            .get("protocols")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Connects this WebSocket to the actual server.
    ///
    /// After calling this method, all messages sent by the page are forwarded to
    /// the server, and all messages sent by the server are forwarded to the page.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocketroute#web-socket-route-connect-to-server>
    pub async fn connect_to_server(&self) -> Result<()> {
        if self.connected.swap(true, Ordering::SeqCst) {
            return Err(crate::error::Error::InvalidArgument(
                "Already connected to the server".to_string(),
            ));
        }
        self.base
            .channel()
            .send_no_result("connect", serde_json::json!({}))
            .await
    }

    /// Whether `connect_to_server` has been called on this route.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Runs after every route handler, as upstream does: a handler that
    /// neither connected to the server nor closed the socket leaves the
    /// page's socket reporting itself open, so the page can send into a
    /// mocked socket.
    pub(crate) async fn after_handle(&self) {
        if self.is_connected() {
            return;
        }
        if let Err(e) = self
            .base
            .channel()
            .send_no_result("ensureOpened", serde_json::json!({}))
            .await
        {
            tracing::debug!("WebSocketRoute ensureOpened failed: {}", e);
        }
    }

    /// Closes the WebSocket connection.
    ///
    /// # Arguments
    ///
    /// * `options` — Optional close code and reason.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocketroute#web-socket-route-close>
    pub async fn close(
        &self,
        options: impl Into<Option<WebSocketRouteCloseOptions>>,
    ) -> Result<()> {
        let options = options.into();
        let opts = options.unwrap_or_default();
        let mut params = serde_json::Map::new();
        if let Some(code) = opts.code {
            params.insert("code".to_string(), serde_json::json!(code));
        }
        if let Some(reason) = opts.reason {
            params.insert("reason".to_string(), serde_json::json!(reason));
        }
        // A close initiated by the handler is reported to the page as a clean
        // close, as upstream's `close()` does.
        params.insert("wasClean".to_string(), serde_json::json!(true));
        self.base
            .channel()
            .send_no_result("closePage", Value::Object(params))
            .await
    }

    /// Sends a text message to the page.
    ///
    /// # Arguments
    ///
    /// * `message` — The text message to send.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocketroute#web-socket-route-send>
    pub async fn send(&self, message: &str) -> Result<()> {
        self.base
            .channel()
            .send_no_result(
                "sendToPage",
                serde_json::json!({ "message": message, "isBase64": false }),
            )
            .await
    }

    /// Registers a handler for messages sent from the page.
    ///
    /// # Arguments
    ///
    /// * `handler` — Async closure that receives the message payload as a `String`.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocketroute#web-socket-route-on-message>
    pub async fn on_message<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(String) -> WebSocketRouteHandlerFuture + Send + Sync + 'static,
    {
        let handler_arc = Arc::new(handler);
        self.message_handlers.lock().unwrap().push(handler_arc);
        Ok(())
    }

    /// Registers a handler for when the WebSocket is closed by the page.
    ///
    /// See: <https://playwright.dev/docs/api/class-websocketroute#web-socket-route-on-close>
    pub async fn on_close<F>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> WebSocketRouteHandlerFuture + Send + Sync + 'static,
    {
        let handler_arc = Arc::new(handler);
        self.close_handlers.lock().unwrap().push(handler_arc);
        Ok(())
    }

    /// Dispatches an incoming server-side event to registered handlers.
    ///
    /// Without a handler the route behaves like a transparent proxy once
    /// `connect_to_server` has been called: page messages go to the server,
    /// server messages and closes go to the page, and a page close closes
    /// the server side. That is upstream's client-side contract; the driver
    /// itself forwards nothing.
    pub(crate) fn handle_event(&self, event: &str, params: &Value) {
        match event {
            "messageFromPage" => {
                let handlers = self.message_handlers.lock().unwrap().clone();
                if handlers.is_empty() {
                    if self.is_connected() {
                        self.forward("sendToServer", params);
                    }
                    return;
                }
                let payload = params["message"].as_str().unwrap_or("").to_string();
                for handler in handlers {
                    let p = payload.clone();
                    tokio::spawn(async move {
                        let _ = handler(p).await;
                    });
                }
            }
            "messageFromServer" => self.forward("sendToPage", params),
            "closePage" => {
                let handlers = self.close_handlers.lock().unwrap().clone();
                if handlers.is_empty() {
                    self.forward("closeServer", params);
                    return;
                }
                for handler in handlers {
                    tokio::spawn(async move {
                        let _ = handler().await;
                    });
                }
            }
            "closeServer" => self.forward("closePage", params),
            _ => {}
        }
    }

    /// Re-sends an event's payload as the mirror-image command; the params
    /// carry the same fields the command takes.
    fn forward(&self, method: &'static str, params: &Value) {
        let channel = self.base.channel().clone();
        let params = params.clone();
        tokio::spawn(async move {
            if let Err(e) = channel.send_no_result(method, params).await {
                tracing::debug!("WebSocketRoute {} forward failed: {}", method, e);
            }
        });
    }
}

/// Options for [`WebSocketRoute::close`].
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct WebSocketRouteCloseOptions {
    /// WebSocket close code (e.g. 1000 for normal closure).
    pub code: Option<u16>,
    /// Human-readable close reason.
    pub reason: Option<String>,
}

impl ChannelOwner for WebSocketRoute {
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
