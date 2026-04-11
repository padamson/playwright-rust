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
//! CDPSession is created via [`BrowserContext::new_cdp_session`].
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
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// A Chrome DevTools Protocol session for a page or browser context.
///
/// CDPSession is only available in Chromium-based browsers.
///
/// See: <https://playwright.dev/docs/api/class-cdpsession>
#[derive(Clone)]
pub struct CDPSession {
    base: ChannelOwnerImpl,
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
        self.base.on_event(method, params)
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
