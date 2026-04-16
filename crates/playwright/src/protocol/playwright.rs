// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Playwright - Root protocol object
//
// Reference:
// - Python: playwright-python/playwright/_impl/_playwright.py
// - Protocol: protocol.yml (Playwright interface)

use crate::error::Result;
use crate::protocol::BrowserType;
use crate::protocol::device::DeviceDescriptor;
use crate::protocol::selectors::Selectors;
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::{ConnectionExt, ConnectionLike};
use crate::server::playwright_server::PlaywrightServer;
use parking_lot::Mutex;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Playwright is the root object that provides access to browser types.
///
/// This is the main entry point for the Playwright API. It provides access to
/// the three browser types (Chromium, Firefox, WebKit) and other top-level services.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::Playwright;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Launch Playwright server and initialize
///     let playwright = Playwright::launch().await?;
///
///     // Verify all three browser types are available
///     let chromium = playwright.chromium();
///     let firefox = playwright.firefox();
///     let webkit = playwright.webkit();
///
///     assert_eq!(chromium.name(), "chromium");
///     assert_eq!(firefox.name(), "firefox");
///     assert_eq!(webkit.name(), "webkit");
///
///     // Verify we can launch a browser
///     let browser = chromium.launch().await?;
///     assert!(!browser.version().is_empty());
///     browser.close().await?;
///
///     // Shutdown when done
///     playwright.shutdown().await?;
///
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-playwright>
#[derive(Clone)]
pub struct Playwright {
    /// Base ChannelOwner implementation
    base: ChannelOwnerImpl,
    /// Chromium browser type
    chromium: BrowserType,
    /// Firefox browser type
    firefox: BrowserType,
    /// WebKit browser type
    webkit: BrowserType,
    /// Playwright server process (for clean shutdown)
    ///
    /// Stored as `Option<PlaywrightServer>` wrapped in Arc<Mutex<>> to allow:
    /// - Sharing across clones (Arc)
    /// - Taking ownership during shutdown (Option::take)
    /// - Interior mutability (Mutex)
    server: Arc<Mutex<Option<PlaywrightServer>>>,
    /// Device descriptors parsed from the initializer's `deviceDescriptors` array.
    devices: HashMap<String, DeviceDescriptor>,
}

impl Playwright {
    /// Launches Playwright and returns a handle to interact with browser types.
    ///
    /// This is the main entry point for the Playwright API. It will:
    /// 1. Launch the Playwright server process
    /// 2. Establish a connection via stdio
    /// 3. Initialize the protocol
    /// 4. Return a Playwright instance with access to browser types
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Playwright server is not found or fails to launch
    /// - Connection to server fails
    /// - Protocol initialization fails
    /// - Server doesn't respond within timeout (30s)
    pub async fn launch() -> Result<Self> {
        use crate::server::connection::Connection;
        use crate::server::playwright_server::PlaywrightServer;
        use crate::server::transport::PipeTransport;

        // 1. Launch Playwright server
        tracing::debug!("Launching Playwright server");
        let mut server = PlaywrightServer::launch().await?;

        // 2. Take stdio streams from server process
        let stdin = server.process.stdin.take().ok_or_else(|| {
            crate::error::Error::ServerError("Failed to get server stdin".to_string())
        })?;

        let stdout = server.process.stdout.take().ok_or_else(|| {
            crate::error::Error::ServerError("Failed to get server stdout".to_string())
        })?;

        // 3. Create transport and connection
        tracing::debug!("Creating transport and connection");
        let (transport, message_rx) = PipeTransport::new(stdin, stdout);
        let (sender, receiver) = transport.into_parts();
        let connection: Arc<Connection> = Arc::new(Connection::new(sender, receiver, message_rx));

        // 4. Spawn connection message loop in background
        let conn_for_loop: Arc<Connection> = Arc::clone(&connection);
        tokio::spawn(async move {
            conn_for_loop.run().await;
        });

        // 5. Initialize Playwright (sends initialize message, waits for Playwright object)
        tracing::debug!("Initializing Playwright protocol");
        let playwright_obj = connection.initialize_playwright().await?;

        // 6. Downcast to Playwright type using get_typed
        let guid = playwright_obj.guid().to_string();
        let mut playwright: Playwright = connection.get_typed::<Playwright>(&guid).await?;

        // Attach the server for clean shutdown
        playwright.server = Arc::new(Mutex::new(Some(server)));

        Ok(playwright)
    }

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
    ///
    /// Note: `Selectors` is a pure client-side coordinator, not a protocol object.
    /// It is created fresh here rather than looked up from the registry.
    pub async fn new(
        connection: Arc<dyn ConnectionLike>,
        type_name: String,
        guid: Arc<str>,
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

        // Get BrowserType objects from connection registry and downcast.
        // Note: These objects should already exist (created by earlier __create__ messages).
        let chromium: BrowserType = connection.get_typed::<BrowserType>(chromium_guid).await?;
        let firefox: BrowserType = connection.get_typed::<BrowserType>(firefox_guid).await?;
        let webkit: BrowserType = connection.get_typed::<BrowserType>(webkit_guid).await?;

        // Selectors is a pure client-side coordinator stored in the connection.
        // No need to create or store it here; access it via self.connection().selectors().

        // Parse deviceDescriptors from LocalUtils.
        //
        // The Playwright initializer has "utils": { "guid": "localUtils" }.
        // LocalUtils's initializer has "deviceDescriptors": [ { "name": "...", "descriptor": { ... } }, ... ]
        //
        // We wrap the inner descriptor fields in a helper struct that matches the
        // server-side shape: { name, descriptor: { userAgent, viewport, ... } }.
        #[derive(serde::Deserialize)]
        struct DeviceEntry {
            name: String,
            descriptor: DeviceDescriptor,
        }

        let local_utils_guid = initializer
            .get("utils")
            .and_then(|v| v.get("guid"))
            .and_then(|v| v.as_str())
            .unwrap_or("localUtils");

        let devices: HashMap<String, DeviceDescriptor> =
            if let Ok(lu) = connection.get_object(local_utils_guid).await {
                lu.initializer()
                    .get("deviceDescriptors")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                serde_json::from_value::<DeviceEntry>(v.clone())
                                    .ok()
                                    .map(|e| (e.name.clone(), e.descriptor))
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                HashMap::new()
            };

        Ok(Self {
            base,
            chromium,
            firefox,
            webkit,
            server: Arc::new(Mutex::new(None)), // No server for protocol-created objects
            devices,
        })
    }

    /// Returns the Chromium browser type.
    pub fn chromium(&self) -> &BrowserType {
        &self.chromium
    }

    /// Returns the Firefox browser type.
    pub fn firefox(&self) -> &BrowserType {
        &self.firefox
    }

    /// Returns the WebKit browser type.
    pub fn webkit(&self) -> &BrowserType {
        &self.webkit
    }

    /// Returns the Selectors object for registering custom selector engines.
    ///
    /// The Selectors instance is shared across all browser contexts created on this
    /// connection. Register custom selector engines here before creating any pages
    /// that will use them.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use playwright_rs::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let selectors = playwright.selectors();
    /// selectors.set_test_id_attribute("data-custom-id").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-playwright#playwright-selectors>
    pub fn selectors(&self) -> std::sync::Arc<Selectors> {
        self.connection().selectors()
    }

    /// Returns the device descriptors map for browser emulation.
    ///
    /// Each entry maps a device name (e.g., `"iPhone 13"`) to a [`DeviceDescriptor`]
    /// containing user agent, viewport, and other emulation settings.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use playwright_rs::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let iphone = &playwright.devices()["iPhone 13"];
    /// // Use iphone fields to configure BrowserContext...
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-playwright#playwright-devices>
    pub fn devices(&self) -> &HashMap<String, DeviceDescriptor> {
        &self.devices
    }

    /// Shuts down the Playwright server gracefully.
    ///
    /// This method should be called when you're done using Playwright to ensure
    /// the server process is terminated cleanly, especially on Windows.
    ///
    /// # Platform-Specific Behavior
    ///
    /// **Windows**: Closes stdio pipes before shutting down to prevent hangs.
    ///
    /// **Unix**: Standard graceful shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if the server shutdown fails.
    pub async fn shutdown(&self) -> Result<()> {
        // Take server from mutex without holding the lock across await
        let server = self.server.lock().take();
        if let Some(server) = server {
            tracing::debug!("Shutting down Playwright server");
            server.shutdown().await?;
        }
        Ok(())
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
        self.base.on_event(method, params)
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for Playwright {
    /// Ensures Playwright server is shut down when Playwright is dropped.
    ///
    /// This is critical on Windows to prevent process hangs when tests complete.
    /// The Drop implementation will attempt to kill the server process synchronously.
    ///
    /// Note: For graceful shutdown, prefer calling `playwright.shutdown().await`
    /// explicitly before dropping.
    fn drop(&mut self) {
        if let Some(mut server) = self.server.lock().take() {
            tracing::debug!("Drop: Force-killing Playwright server");

            // We can't call async shutdown in Drop, so use blocking kill
            // This is less graceful but ensures the process terminates
            #[cfg(windows)]
            {
                // On Windows: Close stdio pipes before killing
                drop(server.process.stdin.take());
                drop(server.process.stdout.take());
                drop(server.process.stderr.take());
            }

            // Force kill the process
            if let Err(e) = server.process.start_kill() {
                tracing::warn!("Failed to kill Playwright server in Drop: {}", e);
            }
        }
    }
}

impl std::fmt::Debug for Playwright {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playwright")
            .field("guid", &self.guid())
            .field("chromium", &self.chromium().name())
            .field("firefox", &self.firefox().name())
            .field("webkit", &self.webkit().name())
            .field("selectors", &*self.selectors())
            .finish()
    }
}

// Note: Playwright testing is done via integration tests since it requires:
// - A real Connection with object registry
// - BrowserType objects already created and registered
// - Protocol messages from the server
// See: crates/playwright-core/tests/connection_integration.rs
