// Browser protocol object
//
// Represents a browser instance created by BrowserType.launch()

use crate::error::Result;
use crate::protocol::{BrowserContext, Page};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

use std::sync::atomic::{AtomicBool, Ordering};

/// Browser represents a browser instance.
///
/// A Browser is created when you call `BrowserType::launch()`. It provides methods
/// to create browser contexts and pages.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::Playwright;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let chromium = playwright.chromium();
///
///     // Launch browser and get info
///     let browser = chromium.launch().await?;
///     println!("Browser: {} version {}", browser.name(), browser.version());
///
///     // Check connection status
///     assert!(browser.is_connected());
///
///     // Create and use contexts and pages
///     let context = browser.new_context().await?;
///     let page = context.new_page().await?;
///
///     // Convenience: create page directly (auto-creates default context)
///     let page2 = browser.new_page().await?;
///
///     // Cleanup
///     browser.close().await?;
///     assert!(!browser.is_connected());
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-browser>
#[derive(Clone)]
pub struct Browser {
    base: ChannelOwnerImpl,
    version: String,
    name: String,
    is_connected: Arc<AtomicBool>,
}

impl Browser {
    /// Creates a new Browser from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Browser object.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent BrowserType object
    /// * `type_name` - The protocol type name ("Browser")
    /// * `guid` - The unique identifier for this browser instance
    /// * `initializer` - The initialization data from the server
    ///
    /// # Errors
    ///
    /// Returns error if initializer is missing required fields (version, name)
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer.clone(),
        );

        let version = initializer["version"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "Browser initializer missing 'version' field".to_string(),
                )
            })?
            .to_string();

        let name = initializer["name"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "Browser initializer missing 'name' field".to_string(),
                )
            })?
            .to_string();

        Ok(Self {
            base,
            version,
            name,
            is_connected: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Returns the browser version string.
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-version>
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the browser name (e.g., "chromium", "firefox", "webkit").
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-name>
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns true if the browser is connected.
    ///
    /// The browser is connected when it is launched and becomes disconnected when:
    /// - `browser.close()` is called
    /// - The browser process crashes
    /// - The browser is closed by the user
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-is-connected>
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    /// Returns the channel for sending protocol messages
    ///
    /// Used internally for sending RPC calls to the browser.
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Creates a new browser context.
    ///
    /// A browser context is an isolated session within the browser instance,
    /// similar to an incognito profile. Each context has its own cookies,
    /// cache, and local storage.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
    pub async fn new_context(&self) -> Result<BrowserContext> {
        // Response contains the GUID of the created BrowserContext
        #[derive(Deserialize)]
        struct NewContextResponse {
            context: GuidRef,
        }

        #[derive(Deserialize)]
        struct GuidRef {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        // Send newContext RPC to server with empty options for now
        let response: NewContextResponse = self
            .channel()
            .send("newContext", serde_json::json!({}))
            .await?;

        // Retrieve the BrowserContext object from the connection registry
        let context_arc = self.connection().get_object(&response.context.guid).await?;

        // Downcast to BrowserContext
        let context = context_arc
            .as_any()
            .downcast_ref::<BrowserContext>()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(format!(
                    "Expected BrowserContext object, got {}",
                    context_arc.type_name()
                ))
            })?;

        Ok(context.clone())
    }

    /// Creates a new browser context with custom options.
    ///
    /// A browser context is an isolated session within the browser instance,
    /// similar to an incognito profile. Each context has its own cookies,
    /// cache, and local storage.
    ///
    /// This method allows customizing viewport, user agent, locale, timezone,
    /// and other settings.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser has been closed
    /// - Communication with browser process fails
    /// - Invalid options provided
    /// - Storage state file cannot be read or parsed
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
    pub async fn new_context_with_options(
        &self,
        mut options: crate::protocol::BrowserContextOptions,
    ) -> Result<BrowserContext> {
        // Response contains the GUID of the created BrowserContext
        #[derive(Deserialize)]
        struct NewContextResponse {
            context: GuidRef,
        }

        #[derive(Deserialize)]
        struct GuidRef {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        // Handle storage_state_path: read file and convert to inline storage_state
        if let Some(path) = &options.storage_state_path {
            let file_content = tokio::fs::read_to_string(path).await.map_err(|e| {
                crate::error::Error::ProtocolError(format!(
                    "Failed to read storage state file '{}': {}",
                    path, e
                ))
            })?;

            let storage_state: crate::protocol::StorageState = serde_json::from_str(&file_content)
                .map_err(|e| {
                    crate::error::Error::ProtocolError(format!(
                        "Failed to parse storage state file '{}': {}",
                        path, e
                    ))
                })?;

            options.storage_state = Some(storage_state);
            options.storage_state_path = None; // Clear path since we've converted to inline
        }

        // Convert options to JSON
        let options_json = serde_json::to_value(options).map_err(|e| {
            crate::error::Error::ProtocolError(format!(
                "Failed to serialize context options: {}",
                e
            ))
        })?;

        // Send newContext RPC to server with options
        let response: NewContextResponse = self.channel().send("newContext", options_json).await?;

        // Retrieve the BrowserContext object from the connection registry
        let context_arc = self.connection().get_object(&response.context.guid).await?;

        // Downcast to BrowserContext
        let context = context_arc
            .as_any()
            .downcast_ref::<BrowserContext>()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(format!(
                    "Expected BrowserContext object, got {}",
                    context_arc.type_name()
                ))
            })?;

        Ok(context.clone())
    }

    /// Creates a new page in a new browser context.
    ///
    /// This is a convenience method that creates a default context and then
    /// creates a page in it. This is equivalent to calling `browser.new_context().await?.new_page().await?`.
    ///
    /// The created context is not directly accessible, but will be cleaned up
    /// when the page is closed.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-page>
    pub async fn new_page(&self) -> Result<Page> {
        // Create a default context and then create a page in it
        let context = self.new_context().await?;
        context.new_page().await
    }

    /// Closes the browser and all of its pages (if any were opened).
    ///
    /// This is a graceful operation that sends a close command to the browser
    /// and waits for it to shut down properly.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser has already been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-close>
    pub async fn close(&self) -> Result<()> {
        // Send close RPC to server
        // The protocol expects an empty object as params
        let result = self
            .channel()
            .send_no_result("close", serde_json::json!({}))
            .await;

        // Add delay on Windows CI to ensure browser process fully terminates
        // This prevents subsequent browser launches from hanging
        #[cfg(windows)]
        {
            let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
            if is_ci {
                tracing::debug!("[playwright-rust] Adding Windows CI browser cleanup delay");
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        result
    }
}

impl ChannelOwner for Browser {
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
        self.is_connected.store(false, Ordering::SeqCst);
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
        if method == "disconnected" {
            self.is_connected.store(false, Ordering::SeqCst);
        }
        self.base.on_event(method, params)
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Browser")
            .field("guid", &self.guid())
            .field("name", &self.name)
            .field("version", &self.version)
            .finish()
    }
}

// Note: Browser testing is done via integration tests since it requires:
// - A real Connection with object registry
// - Protocol messages from the server
// - BrowserType.launch() to create Browser objects
// See: crates/playwright-core/tests/browser_launch_integration.rs (Phase 2 Slice 3)
