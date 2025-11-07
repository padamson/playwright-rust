// Browser protocol object
//
// Represents a browser instance created by BrowserType.launch()

use crate::channel::Channel;
use crate::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::error::Result;
use crate::protocol::{BrowserContext, Page};
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// Browser represents a browser instance.
///
/// A Browser is created when you call [`BrowserType::launch()`]. It provides methods
/// to create browser contexts and pages.
///
/// # Example
///
/// ```no_run
/// use playwright_core::protocol::Playwright;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let chromium = playwright.chromium();
///
///     // Launch browser
///     let browser = chromium.launch().await?;
///
///     println!("Browser: {} version {}", browser.name(), browser.version());
///
///     // Close browser
///     browser.close().await?;
///
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
        guid: String,
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
        })
    }

    /// Returns the browser version string
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// println!("Browser version: {}", browser.version());
    /// # Ok(())
    /// # }
    /// ```
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the browser name (e.g., "chromium", "firefox", "webkit")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// assert_eq!(browser.name(), "chromium");
    /// # Ok(())
    /// # }
    /// ```
    pub fn name(&self) -> &str {
        &self.name
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
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    ///
    /// // Create an isolated context
    /// let context = browser.new_context().await?;
    ///
    /// // Do work with context...
    ///
    /// // Cleanup
    /// context.close().await?;
    /// browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
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
            guid: String,
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

    /// Creates a new page in a new browser context.
    ///
    /// This is a convenience method that creates a default context and then
    /// creates a page in it. This is equivalent to calling `browser.new_context().await?.new_page().await?`.
    ///
    /// The created context is not directly accessible, but will be cleaned up
    /// when the page is closed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    ///
    /// // Create page directly (creates default context automatically)
    /// let page = browser.new_page().await?;
    ///
    /// // Do work with page...
    ///
    /// // Cleanup
    /// page.close().await?;
    /// browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
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
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    ///
    /// // Do work with browser...
    ///
    /// // Close browser when done
    /// browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
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
        self.channel()
            .send_no_result("close", serde_json::json!({}))
            .await
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

    fn connection(&self) -> Arc<dyn crate::connection::ConnectionLike> {
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

    fn on_event(&self, _method: &str, _params: Value) {
        // TODO: Handle browser events in future phases
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
