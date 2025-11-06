// Copyright 2024 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// BrowserType - Represents a browser type (Chromium, Firefox, WebKit)
//
// Reference:
// - Python: playwright-python/playwright/_impl/_browser_type.py
// - Protocol: protocol.yml (BrowserType interface)

use crate::channel::Channel;
use crate::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::connection::ConnectionLike;
use crate::error::Result;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// BrowserType represents a browser engine (Chromium, Firefox, or WebKit).
///
/// Each Playwright instance provides three BrowserType objects accessible via:
/// - `playwright.chromium()`
/// - `playwright.firefox()`
/// - `playwright.webkit()`
///
/// # Example
///
/// ```no_run
/// # use playwright_core::protocol::BrowserType;
/// # async fn example(chromium: &BrowserType) -> Result<(), Box<dyn std::error::Error>> {
/// // Browser launching will be implemented in Phase 2
/// println!("Browser: {}", chromium.name());
/// println!("Executable: {}", chromium.executable_path());
/// # Ok(())
/// # }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-browsertype>
pub struct BrowserType {
    /// Base ChannelOwner implementation
    base: ChannelOwnerImpl,
    /// Browser name ("chromium", "firefox", or "webkit")
    name: String,
    /// Path to browser executable
    executable_path: String,
}

impl BrowserType {
    /// Creates a new BrowserType object from protocol initialization.
    ///
    /// Called by the object factory when server sends __create__ message.
    ///
    /// # Arguments
    /// * `parent` - Parent Playwright object
    /// * `type_name` - Protocol type name ("BrowserType")
    /// * `guid` - Unique GUID from server (e.g., "browserType@chromium")
    /// * `initializer` - Initial state with name and executablePath
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: String,
        initializer: Value,
    ) -> Result<Self> {
        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer.clone(),
        );

        // Extract fields from initializer
        let name = initializer["name"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "BrowserType initializer missing 'name'".to_string(),
                )
            })?
            .to_string();

        let executable_path = initializer["executablePath"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "BrowserType initializer missing 'executablePath'".to_string(),
                )
            })?
            .to_string();

        Ok(Self {
            base,
            name,
            executable_path,
        })
    }

    /// Returns the browser name ("chromium", "firefox", or "webkit").
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the path to the browser executable.
    pub fn executable_path(&self) -> &str {
        &self.executable_path
    }

    // TODO: Phase 2 - Add launch() method for launching browsers
}

impl ChannelOwner for BrowserType {
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

impl std::fmt::Debug for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserType")
            .field("guid", &self.guid())
            .field("name", &self.name)
            .field("executable_path", &self.executable_path)
            .finish()
    }
}

// Note: BrowserType testing is done via integration tests since it requires:
// - A real Connection with object registry
// - Protocol messages from the server
// See: crates/playwright-core/tests/connection_integration.rs
