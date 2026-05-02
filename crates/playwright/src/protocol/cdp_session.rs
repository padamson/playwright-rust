// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// CDPSession — Chrome DevTools Protocol session object
//
// Architecture Reference:
// - Python: playwright-python/playwright/_impl/_cdp_session.py
// - JavaScript: playwright/packages/playwright-core/src/client/cdpSession.ts
// - Docs: https://playwright.dev/docs/api/class-cdpsession

//! CDPSession — Chrome DevTools Protocol session
//!
//! Provides access to the Chrome DevTools Protocol for Chromium-based browsers.
//! CDPSession is created via [`BrowserContext::new_cdp_session`](crate::protocol::BrowserContext::new_cdp_session).
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
//!     let context = browser.new_context().await?;
//!     let page = context.new_page().await?;
//!
//!     // Create a CDP session for the page
//!     let session = context.new_cdp_session(&page).await?;
//!
//!     // Send a CDP command
//!     let result = session
//!         .send("Runtime.evaluate", Some(serde_json::json!({ "expression": "1+1" })))
//!         .await?;
//!
//!     println!("Result: {:?}", result);
//!
//!     session.detach().await?;
//!     context.close().await?;
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-cdpsession>

use crate::error::Result;
use crate::server::channel::Channel;
use crate::server::channel_owner::{
    ChannelOwner, ChannelOwnerImpl, DisposeReason, ParentOrConnection,
};
use crate::server::connection::ConnectionLike;
use parking_lot::Mutex;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type EventHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
type EventHandler = Arc<dyn Fn(Value) -> EventHandlerFuture + Send + Sync + 'static>;
type CloseHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
type CloseHandler = Arc<dyn Fn() -> CloseHandlerFuture + Send + Sync + 'static>;

/// A Chrome DevTools Protocol session for a page or browser context.
///
/// CDPSession is only available in Chromium-based browsers.
///
/// See: <https://playwright.dev/docs/api/class-cdpsession>
#[derive(Clone)]
pub struct CDPSession {
    base: ChannelOwnerImpl,
    event_handlers: Arc<Mutex<HashMap<String, Vec<EventHandler>>>>,
    close_handlers: Arc<Mutex<Vec<CloseHandler>>>,
}

impl CDPSession {
    /// Creates a new CDPSession from protocol initialization.
    ///
    /// Called by the object factory when the server sends a `__create__` message.
    pub fn new(
        parent: ParentOrConnection,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        Ok(Self {
            base: ChannelOwnerImpl::new(parent, type_name, guid, initializer),
            event_handlers: Arc::new(Mutex::new(HashMap::new())),
            close_handlers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Send a CDP command and return the result.
    ///
    /// # Arguments
    ///
    /// * `method` - The CDP method name (e.g., `"Runtime.evaluate"`)
    /// * `params` - Optional JSON parameters for the method
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The session has been detached
    /// - The CDP method fails
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-cdpsession#cdp-session-send>
    pub async fn send(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let params = serde_json::json!({
            "method": method,
            "params": params.unwrap_or(serde_json::json!({})),
        });
        self.channel().send("send", params).await
    }

    /// Register a handler for a CDP event by method name. Multiple handlers
    /// may be registered for the same method; they fire in registration
    /// order. The Playwright server forwards every CDP event the
    /// underlying session emits — no Playwright-side subscription is
    /// needed, but you typically must enable the CDP domain itself first
    /// (e.g. `session.send("Network.enable", None).await?` before
    /// expecting `Network.requestWillBeSent`).
    ///
    /// See: <https://playwright.dev/docs/api/class-cdpsession#cdp-session-on>
    pub fn on<F, Fut>(&self, method: impl Into<String>, handler: F)
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let h: EventHandler =
            Arc::new(move |v: Value| -> EventHandlerFuture { Box::pin(handler(v)) });
        self.event_handlers
            .lock()
            .entry(method.into())
            .or_default()
            .push(h);
    }

    /// Register a handler for the `close` event, fired when the session
    /// is detached (parent target closes, browser closes, or
    /// [`detach`](Self::detach) is called).
    ///
    /// See: <https://playwright.dev/docs/api/class-cdpsession#cdp-session-event-close>
    pub fn on_close<F, Fut>(&self, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let h: CloseHandler = Arc::new(move || -> CloseHandlerFuture { Box::pin(handler()) });
        self.close_handlers.lock().push(h);
    }

    /// Detach the CDP session from the target.
    ///
    /// After detaching, the session can no longer be used to send commands.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The session has already been detached
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-cdpsession#cdp-session-detach>
    pub async fn detach(&self) -> Result<()> {
        self.channel()
            .send_no_result("detach", serde_json::json!({}))
            .await
    }
}

impl ChannelOwner for CDPSession {
    fn guid(&self) -> &str {
        self.base.guid()
    }

    fn type_name(&self) -> &str {
        self.base.type_name()
    }

    fn parent(&self) -> Option<Arc<dyn ChannelOwner>> {
        self.base.parent()
    }

    fn connection(&self) -> Arc<dyn ConnectionLike> {
        self.base.connection()
    }

    fn initializer(&self) -> &Value {
        self.base.initializer()
    }

    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    fn dispose(&self, reason: DisposeReason) {
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
        match method {
            "event" => {
                let cdp_method = params
                    .get("method")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let cdp_params = params.get("params").cloned().unwrap_or(Value::Null);
                if let Some(cdp_method) = cdp_method {
                    let handlers = self
                        .event_handlers
                        .lock()
                        .get(&cdp_method)
                        .cloned()
                        .unwrap_or_default();
                    for h in handlers {
                        let p = cdp_params.clone();
                        tokio::spawn(async move {
                            if let Err(e) = h(p).await {
                                tracing::warn!("CDPSession event handler error: {}", e);
                            }
                        });
                    }
                }
            }
            "close" => {
                let handlers = self.close_handlers.lock().clone();
                for h in handlers {
                    tokio::spawn(async move {
                        if let Err(e) = h().await {
                            tracing::warn!("CDPSession close handler error: {}", e);
                        }
                    });
                }
            }
            _ => {}
        }
        self.base.on_event(method, params);
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for CDPSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CDPSession")
            .field("guid", &self.guid())
            .finish()
    }
}
