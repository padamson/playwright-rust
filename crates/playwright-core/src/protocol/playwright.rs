// Copyright 2024 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Playwright - Root protocol object
//
// Reference:
// - Python: playwright-python/playwright/_impl/_playwright.py
// - Protocol: protocol.yml (Playwright interface)

use crate::channel::Channel;
use crate::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::connection::ConnectionLike;
use crate::error::Result;
use crate::protocol::BrowserType;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// Playwright is the root object that provides access to browser types.
///
/// This is the main entry point for the Playwright API. It provides access to
/// the three browser types (Chromium, Firefox, WebKit) and other top-level services.
///
/// # Example
///
/// ```no_run
/// # use playwright_core::protocol::Playwright;
/// # async fn example(playwright: &Playwright) -> Result<(), Box<dyn std::error::Error>> {
/// // Access browser types (Phase 1 - objects exist but browsers not launched yet)
/// let chromium = playwright.chromium();
/// let firefox = playwright.firefox();
/// let webkit = playwright.webkit();
///
/// println!("Chromium: {}", chromium.name());
/// println!("Firefox: {}", firefox.name());
/// println!("WebKit: {}", webkit.name());
/// # Ok(())
/// # }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-playwright>
pub struct Playwright {
    /// Base ChannelOwner implementation
    base: ChannelOwnerImpl,
    /// Chromium browser type (stored as Arc<dyn ChannelOwner>, downcast on access)
    chromium: Arc<dyn ChannelOwner>,
    /// Firefox browser type (stored as Arc<dyn ChannelOwner>, downcast on access)
    firefox: Arc<dyn ChannelOwner>,
    /// WebKit browser type (stored as Arc<dyn ChannelOwner>, downcast on access)
    webkit: Arc<dyn ChannelOwner>,
}

impl Playwright {
    /// Creates a new Playwright object from protocol initialization.
    ///
    /// Called by the object factory when server sends __create__ message for root object.
    ///
    /// # Arguments
    /// * `connection` - The connection (Playwright is root, so no parent)
    /// * `type_name` - Protocol type name ("Playwright")
    /// * `guid` - Unique GUID from server (typically "playwright@1")
    /// * `initializer` - Initial state with references to browser types
    ///
    /// # Initializer Format
    ///
    /// The initializer contains GUID references to BrowserType objects:
    /// ```json
    /// {
    ///   "chromium": { "guid": "browserType@chromium" },
    ///   "firefox": { "guid": "browserType@firefox" },
    ///   "webkit": { "guid": "browserType@webkit" }
    /// }
    /// ```
    pub fn new(
        connection: Arc<dyn ConnectionLike>,
        type_name: String,
        guid: String,
        initializer: Value,
    ) -> Result<Self> {
        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Connection(connection.clone()),
            type_name,
            guid,
            initializer.clone(),
        );

        // Extract BrowserType GUIDs from initializer
        let chromium_guid = initializer["chromium"]["guid"].as_str().ok_or_else(|| {
            crate::error::Error::ProtocolError(
                "Playwright initializer missing 'chromium.guid'".to_string(),
            )
        })?;

        let firefox_guid = initializer["firefox"]["guid"].as_str().ok_or_else(|| {
            crate::error::Error::ProtocolError(
                "Playwright initializer missing 'firefox.guid'".to_string(),
            )
        })?;

        let webkit_guid = initializer["webkit"]["guid"].as_str().ok_or_else(|| {
            crate::error::Error::ProtocolError(
                "Playwright initializer missing 'webkit.guid'".to_string(),
            )
        })?;

        // Get BrowserType objects from connection registry
        // Note: These objects should already exist (created by earlier __create__ messages)
        // We store them as Arc<dyn ChannelOwner> and downcast when accessed
        let chromium = connection.get_object(chromium_guid)?;
        let firefox = connection.get_object(firefox_guid)?;
        let webkit = connection.get_object(webkit_guid)?;

        Ok(Self {
            base,
            chromium,
            firefox,
            webkit,
        })
    }

    /// Returns the Chromium browser type.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # async fn example(playwright: &Playwright) {
    /// let chromium = playwright.chromium();
    /// println!("Browser: {}", chromium.name());
    /// # }
    /// ```
    pub fn chromium(&self) -> &BrowserType {
        // Downcast from Arc<dyn ChannelOwner> to &BrowserType
        self.chromium
            .as_any()
            .downcast_ref::<BrowserType>()
            .expect("chromium should be BrowserType")
    }

    /// Returns the Firefox browser type.
    pub fn firefox(&self) -> &BrowserType {
        self.firefox
            .as_any()
            .downcast_ref::<BrowserType>()
            .expect("firefox should be BrowserType")
    }

    /// Returns the WebKit browser type.
    pub fn webkit(&self) -> &BrowserType {
        self.webkit
            .as_any()
            .downcast_ref::<BrowserType>()
            .expect("webkit should be BrowserType")
    }
}

impl ChannelOwner for Playwright {
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

    fn dispose(&self, reason: crate::channel_owner::DisposeReason) {
        self.base.dispose(reason)
    }

    fn adopt(&self, child: Arc<dyn ChannelOwner>) {
        self.base.adopt(child)
    }

    fn add_child(&self, guid: String, child: Arc<dyn ChannelOwner>) {
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

impl std::fmt::Debug for Playwright {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playwright")
            .field("guid", &self.guid())
            .field("chromium", &self.chromium().name())
            .field("firefox", &self.firefox().name())
            .field("webkit", &self.webkit().name())
            .finish()
    }
}

// Note: Playwright testing is done via integration tests since it requires:
// - A real Connection with object registry
// - BrowserType objects already created and registered
// - Protocol messages from the server
// See: crates/playwright-core/tests/connection_integration.rs
