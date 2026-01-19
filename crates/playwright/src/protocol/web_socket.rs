// WebSocket protocol object
//
// Represents a WebSocket connection in the page.
//
// # Example
//
// ```ignore
// use playwright_rs::protocol::{Playwright, WebSocket};
//
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let playwright = Playwright::launch().await?;
//     let browser = playwright.chromium().launch().await?;
//     let page = browser.new_page().await?;
//
//     // Listen for WebSocket connections
//     page.on_websocket(|ws| {
//         println!("WebSocket opened: {}", ws.url());
//
//         // Listen for frames
//         let ws_clone = ws.clone();
//         Box::pin(async move {
//             ws_clone.on_frame_received(|payload| {
//                 Box::pin(async move {
//                     println!("Received: {:?}", payload);
//                     Ok(())
//                 })
//             }).await?;
//             Ok(())
//         })
//     }).await?;
//
//     page.goto("https://websocket.org/echo.html", None).await?;
//
//     browser.close().await?;
//     Ok(())
// }
// ```

use crate::error::Result;
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde_json::Value;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// WebSocket represents a WebSocket connection in the page.
#[derive(Clone)]
pub struct WebSocket {
    base: ChannelOwnerImpl,
    url: String,
    // Event handlers
    handlers: Arc<Mutex<Vec<WebSocketEventHandler>>>,
}

/// Type alias for boxed event handler future
type WebSocketEventHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// WebSocket event handler
type WebSocketEventHandler =
    Arc<dyn Fn(WebSocketEvent) -> WebSocketEventHandlerFuture + Send + Sync>;

#[derive(Clone, Debug)]
enum WebSocketEvent {
    FrameSent(String), // Payload is string (text) or base64 (binary)
    FrameReceived(String),
    SocketError(String),
    Close,
}

impl WebSocket {
    /// Creates a new WebSocket object
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

        let handlers = Arc::new(Mutex::new(Vec::new()));

        Ok(Self {
            base,
            url,
            handlers,
        })
    }

    /// Returns the URL of the WebSocket.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns true if the WebSocket is closed.
    pub fn is_closed(&self) -> bool {
        // Simple check based on basic close event tracking could be added here
        // For now, we rely on the protocol state or user tracking
        false
    }

    /// Adds a listener for FrameSent events.
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

    /// Adds a listener for FrameReceived events.
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

    /// Adds a listener for SocketError events.
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

    /// Adds a listener for Close events.
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

    // Dispatch methods required by the protocol layer
    // These are called when the server sends an event

    pub(crate) fn handle_event(&self, event: &str, params: &Value) {
        let ws_event = match event {
            "frameSent" => {
                let _payload = params["opcode"].as_i64().map_or("".to_string(), |op| {
                    if op == 2 {
                        // Binary
                        params["data"].as_str().unwrap_or("").to_string()
                    } else {
                        // Text
                        params["data"].as_str().unwrap_or("").to_string()
                    }
                });
                // Simplified: Just returning data for now
                WebSocketEvent::FrameSent(params["data"].as_str().unwrap_or("").to_string())
            }
            "frameReceived" => {
                WebSocketEvent::FrameReceived(params["data"].as_str().unwrap_or("").to_string())
            }
            "socketError" => {
                WebSocketEvent::SocketError(params["error"].as_str().unwrap_or("").to_string())
            }
            "close" => WebSocketEvent::Close,
            _ => return,
        };

        let handlers = self.handlers.lock().unwrap();
        for handler in handlers.iter() {
            let handler = handler.clone();
            let event = ws_event.clone();
            // Fire and forget
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
