// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Root - Internal object for sending initialize message
//
// Reference:
// - Python: playwright-python/playwright/_impl/_connection.py (RootChannelOwner)
// - Java: playwright-java/.../impl/Connection.java (Root inner class)
// - .NET: playwright-dotnet/src/Playwright/Transport/Connection.cs (InitializePlaywrightAsync)

use crate::error::Result;
use crate::server::channel::Channel;
use crate::server::channel_owner::{
    ChannelOwner, ChannelOwnerImpl, DisposeReason, ParentOrConnection,
};
use crate::server::connection::ConnectionLike;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// Root object for sending the initialize message to the Playwright server
///
/// This is an internal object not exposed to end users. It exists solely to
/// send the `initialize` message to the server during connection setup.
///
/// # Protocol Flow
///
/// When `initialize()` is called:
/// 1. Sends `initialize` message with `sdkLanguage: "rust"`
/// 2. Server creates BrowserType objects (sends `__create__` messages)
/// 3. Server creates Playwright object (sends `__create__` message)
/// 4. Server responds with Playwright GUID: `{ "playwright": { "guid": "..." } }`
/// 5. All objects are now in the connection's object registry
///
/// The Root object has an empty GUID (`""`) and is not registered in the
/// object registry. It's discarded after initialization completes.
///
/// # Example
///
/// ```ignore
/// # use playwright_rs::protocol::Root;
/// # use playwright_rs::server::connection::ConnectionLike;
/// # use std::sync::Arc;
/// # async fn example(connection: Arc<dyn ConnectionLike>) -> Result<(), Box<dyn std::error::Error>> {
/// // Create root object with connection
/// let root = Root::new(connection.clone());
///
/// // Send initialize message to server
/// let response = root.initialize().await?;
///
/// // Verify Playwright GUID is returned
/// let playwright_guid = response["playwright"]["guid"]
///     .as_str()
///     .expect("Missing playwright.guid");
/// assert!(!playwright_guid.is_empty());
/// assert!(playwright_guid.contains("playwright"));
///
/// // Verify response contains BrowserType objects
/// assert!(response["playwright"].is_object());
/// # Ok(())
/// # }
/// ```
///
/// See:
/// - Python: <https://github.com/microsoft/playwright-python/blob/main/playwright/_impl/_connection.py>
/// - Java: <https://github.com/microsoft/playwright-java>
pub struct Root {
    /// Base ChannelOwner implementation
    base: ChannelOwnerImpl,
}

impl Root {
    /// Creates a new Root object
    ///
    /// # Arguments
    ///
    /// * `connection` - The connection to the Playwright server
    pub fn new(connection: Arc<dyn ConnectionLike>) -> Self {
        Self {
            base: ChannelOwnerImpl::new(
                ParentOrConnection::Connection(connection),
                "Root".to_string(),
                Arc::from(""), // Empty GUID - Root is not registered in object map
                Value::Null,
            ),
        }
    }

    /// Send the initialize message to the Playwright server
    ///
    /// This is a synchronous request that blocks until the server responds.
    /// By the time the response arrives, all protocol objects (Playwright,
    /// BrowserType, etc.) will have been created and registered.
    ///
    /// # Returns
    ///
    /// The server response containing the Playwright object GUID:
    /// ```json
    /// {
    ///   "playwright": {
    ///     "guid": "playwright"
    ///   }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Message send fails
    /// - Server returns protocol error
    /// - Connection is closed
    pub async fn initialize(&self) -> Result<Value> {
        self.channel()
            .send(
                "initialize",
                serde_json::json!({
                    // TODO: Use "rust" once upstream Playwright accepts it
                    // Current issue: Playwright v1.49.0 protocol validator only accepts:
                    // (javascript|python|java|csharp)
                    //
                    // Using "python" because:
                    // - Closest async/await patterns to Rust
                    // - sdkLanguage only affects CLI error messages and codegen
                    // - Does NOT affect core protocol functionality
                    // - Python error messages are appropriate ("playwright install")
                    //
                    // Plan: Contribute to microsoft/playwright to add 'rust' to Language enum
                    // See: packages/playwright-core/src/utils/isomorphic/locatorGenerators.ts
                    "sdkLanguage": "python"
                }),
            )
            .await
    }
}

impl ChannelOwner for Root {
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

impl std::fmt::Debug for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Root")
            .field("guid", &self.guid())
            .field("type_name", &self.type_name())
            .finish()
    }
}

// Note: Root object testing is done via integration tests since it requires:
// - A real Connection to send messages
// - A real Playwright server to respond
// See: crates/playwright-core/tests/initialization_integration.rs
