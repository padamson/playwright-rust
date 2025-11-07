// Page protocol object
//
// Represents a web page within a browser context.
// Pages are isolated tabs or windows within a context.

use crate::channel::Channel;
use crate::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::error::Result;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// Page represents a web page within a browser context.
///
/// A Page is created when you call [`BrowserContext::new_page()`] or [`Browser::new_page()`].
/// Each page is an isolated tab/window within its parent context.
///
/// Initially, pages are navigated to "about:blank". Use navigation methods
/// (implemented in Phase 3) to navigate to URLs.
///
/// # Example
///
/// ```no_run
/// # use playwright_core::protocol::Playwright;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let playwright = Playwright::launch().await?;
/// let browser = playwright.chromium().launch().await?;
/// let context = browser.new_context().await?;
///
/// // Create a page
/// let page = context.new_page().await?;
///
/// // Page starts at about:blank
/// assert_eq!(page.url(), "about:blank");
///
/// // Cleanup
/// page.close().await?;
/// context.close().await?;
/// browser.close().await?;
/// # Ok(())
/// # }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-page>
#[derive(Clone)]
pub struct Page {
    base: ChannelOwnerImpl,
}

impl Page {
    /// Creates a new Page from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Page object.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent BrowserContext object
    /// * `type_name` - The protocol type name ("Page")
    /// * `guid` - The unique identifier for this page
    /// * `initializer` - The initialization data from the server
    ///
    /// # Errors
    ///
    /// Returns error if initializer is malformed
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
            initializer,
        );

        Ok(Self { base })
    }

    /// Returns the channel for sending protocol messages
    ///
    /// Used internally for sending RPC calls to the page.
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Returns the current URL of the page.
    ///
    /// This returns the last committed URL. Initially, pages are at "about:blank".
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// // Initially at about:blank
    /// assert_eq!(page.url(), "about:blank");
    ///
    /// // After navigation (Phase 3)
    /// // page.goto("https://example.com").await?;
    /// // assert_eq!(page.url(), "https://example.com/");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-url>
    pub fn url(&self) -> &str {
        // The URL is stored in the mainFrame's initializer
        // For Phase 2, we'll return "about:blank" as pages start there
        // In Phase 3, we'll track URL changes through events
        "about:blank"
    }

    /// Closes the page.
    ///
    /// This is a graceful operation that sends a close command to the page
    /// and waits for it to shut down properly.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    ///
    /// // Do work with page...
    ///
    /// // Close page when done
    /// page.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has already been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-close>
    pub async fn close(&self) -> Result<()> {
        // Send close RPC to server
        self.channel()
            .send_no_result("close", serde_json::json!({}))
            .await
    }
}

impl ChannelOwner for Page {
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
        // TODO: Handle page events in future phases
        // Events: load, domcontentloaded, close, crash, etc.
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Page")
            .field("guid", &self.guid())
            .field("url", &self.url())
            .finish()
    }
}
