// ElementHandle protocol object
//
// Represents a DOM element in the page. Supports element-specific operations like screenshots.
// ElementHandles are created via query_selector methods and are protocol objects with GUIDs.

use crate::error::Result;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use base64::Engine;
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// ElementHandle represents a DOM element in the page.
///
/// ElementHandles are created via `page.query_selector()` or `frame.query_selector()`.
/// They are protocol objects that allow element-specific operations like taking screenshots.
///
/// See: <https://playwright.dev/docs/api/class-elementhandle>
#[derive(Clone)]
pub struct ElementHandle {
    base: ChannelOwnerImpl,
}

impl ElementHandle {
    /// Creates a new ElementHandle from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for an ElementHandle object.
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
            initializer,
        );

        Ok(Self { base })
    }

    /// Takes a screenshot of the element and returns the image bytes.
    ///
    /// The screenshot is captured as PNG by default.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use playwright_rs::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    /// page.goto("https://example.com", None).await?;
    ///
    /// let element = page.query_selector("h1").await?.expect("h1 not found");
    /// let screenshot_bytes = element.screenshot(None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-elementhandle#element-handle-screenshot>
    pub async fn screenshot(
        &self,
        options: Option<crate::protocol::ScreenshotOptions>,
    ) -> Result<Vec<u8>> {
        let params = if let Some(opts) = options {
            opts.to_json()
        } else {
            // Default to PNG with required timeout
            serde_json::json!({
                "type": "png",
                "timeout": crate::DEFAULT_TIMEOUT_MS
            })
        };

        #[derive(Deserialize)]
        struct ScreenshotResponse {
            binary: String,
        }

        let response: ScreenshotResponse = self.base.channel().send("screenshot", params).await?;

        // Decode base64 to bytes
        let bytes = base64::prelude::BASE64_STANDARD
            .decode(&response.binary)
            .map_err(|e| {
                crate::error::Error::ProtocolError(format!(
                    "Failed to decode element screenshot: {}",
                    e
                ))
            })?;

        Ok(bytes)
    }
}

impl ChannelOwner for ElementHandle {
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

    fn channel(&self) -> &crate::server::channel::Channel {
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

    fn on_event(&self, _method: &str, _params: Value) {
        // ElementHandle events will be handled in future phases if needed
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for ElementHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElementHandle")
            .field("guid", &self.guid())
            .finish()
    }
}
