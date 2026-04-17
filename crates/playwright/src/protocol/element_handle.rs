// ElementHandle protocol object
//
// Represents a DOM element in the page. Supports element-specific operations like screenshots.
// ElementHandles are created via query_selector methods and are protocol objects with GUIDs.

use crate::error::Result;
use crate::protocol::locator::BoundingBox;
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

    /// Returns the bounding box of this element, or None if it is not visible.
    ///
    /// The bounding box is in pixels, relative to the top-left corner of the page.
    ///
    /// See: <https://playwright.dev/docs/api/class-elementhandle#element-handle-bounding-box>
    pub async fn bounding_box(&self) -> Result<Option<BoundingBox>> {
        #[derive(Deserialize)]
        struct BoundingBoxResponse {
            value: Option<BoundingBox>,
        }

        let response: BoundingBoxResponse = self
            .base
            .channel()
            .send(
                "boundingBox",
                serde_json::json!({
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Sets files on this element (which must be an `<input type="file">`).
    ///
    /// Called by [`FileChooser::set_files`](crate::protocol::FileChooser::set_files) to
    /// satisfy a file chooser dialog by setting files directly on the element.
    ///
    /// # Arguments
    ///
    /// * `files` - Slice of file paths to set on the input element
    ///
    /// See: <https://playwright.dev/docs/api/class-filechooser#file-chooser-set-files>
    pub(crate) async fn set_input_files(
        &self,
        files: &[std::path::PathBuf],
    ) -> crate::error::Result<()> {
        use base64::{Engine as _, engine::general_purpose};

        let payloads: Vec<serde_json::Value> = files
            .iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "file".to_string());
                let mime_type = mime_guess::from_path(path)
                    .first_or_octet_stream()
                    .to_string();
                let buffer = std::fs::read(path).unwrap_or_default();
                let b64 = general_purpose::STANDARD.encode(&buffer);
                serde_json::json!({
                    "name": name,
                    "mimeType": mime_type,
                    "buffer": b64
                })
            })
            .collect();

        self.base
            .channel()
            .send_no_result(
                "setInputFiles",
                serde_json::json!({
                    "payloads": payloads,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await
    }

    /// Scrolls this element into the viewport if it is not already visible.
    ///
    /// See: <https://playwright.dev/docs/api/class-elementhandle#element-handle-scroll-into-view-if-needed>
    pub async fn scroll_into_view_if_needed(&self) -> Result<()> {
        self.base
            .channel()
            .send_no_result(
                "scrollIntoViewIfNeeded",
                serde_json::json!({
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await
    }

    /// Returns the `Frame` associated with this `<iframe>` element, or `None` if
    /// the element is not an iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-elementhandle#element-handle-content-frame>
    pub async fn content_frame(&self) -> Result<Option<crate::protocol::Frame>> {
        use crate::server::connection::ConnectionExt;

        #[derive(Deserialize)]
        struct FrameRef {
            guid: String,
        }
        #[derive(Deserialize)]
        struct ContentFrameResponse {
            frame: Option<FrameRef>,
        }

        let response: ContentFrameResponse = self
            .base
            .channel()
            .send("contentFrame", serde_json::json!({}))
            .await?;

        match response.frame {
            None => Ok(None),
            Some(frame_ref) => {
                let connection = self.base.connection();
                let frame = connection
                    .get_typed::<crate::protocol::Frame>(&frame_ref.guid)
                    .await?;
                Ok(Some(frame))
            }
        }
    }

    /// Returns the `Frame` that owns this element.
    ///
    /// Every element belongs to a frame (the main frame or a child iframe frame).
    ///
    /// See: <https://playwright.dev/docs/api/class-elementhandle#element-handle-owner-frame>
    pub async fn owner_frame(&self) -> Result<Option<crate::protocol::Frame>> {
        use crate::server::connection::ConnectionExt;

        #[derive(Deserialize)]
        struct FrameRef {
            guid: String,
        }
        #[derive(Deserialize)]
        struct OwnerFrameResponse {
            frame: Option<FrameRef>,
        }

        let response: OwnerFrameResponse = self
            .base
            .channel()
            .send("ownerFrame", serde_json::json!({}))
            .await?;

        match response.frame {
            None => Ok(None),
            Some(frame_ref) => {
                let connection = self.base.connection();
                let frame = connection
                    .get_typed::<crate::protocol::Frame>(&frame_ref.guid)
                    .await?;
                Ok(Some(frame))
            }
        }
    }

    /// Waits until the element reaches the specified state.
    ///
    /// Valid states: `"visible"`, `"hidden"`, `"stable"`, `"enabled"`, `"disabled"`, `"editable"`.
    ///
    /// # Arguments
    ///
    /// * `state` — the element state to wait for
    /// * `timeout` — optional timeout in milliseconds (defaults to [`DEFAULT_TIMEOUT_MS`](crate::DEFAULT_TIMEOUT_MS))
    ///
    /// See: <https://playwright.dev/docs/api/class-elementhandle#element-handle-wait-for-element-state>
    pub async fn wait_for_element_state(&self, state: &str, timeout: Option<f64>) -> Result<()> {
        let timeout_ms = timeout.unwrap_or(crate::DEFAULT_TIMEOUT_MS);
        self.base
            .channel()
            .send_no_result(
                "waitForElementState",
                serde_json::json!({
                    "state": state,
                    "timeout": timeout_ms
                }),
            )
            .await
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
