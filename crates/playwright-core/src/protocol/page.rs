// Page protocol object
//
// Represents a web page within a browser context.
// Pages are isolated tabs or windows within a context.

use crate::channel::Channel;
use crate::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::error::Result;
use base64::Engine;
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::sync::{Arc, RwLock};

/// Page represents a web page within a browser context.
///
/// A Page is created when you call `BrowserContext::new_page()` or `Browser::new_page()`.
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
    /// Current URL of the page
    /// Wrapped in RwLock to allow updates from events
    url: Arc<RwLock<String>>,
    /// GUID of the main frame
    main_frame_guid: String,
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
        // Extract mainFrame GUID from initializer
        let main_frame_guid = initializer["mainFrame"]["guid"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "Page initializer missing 'mainFrame.guid' field".to_string(),
                )
            })?
            .to_string();

        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer,
        );

        // Initialize URL to about:blank
        let url = Arc::new(RwLock::new("about:blank".to_string()));

        Ok(Self {
            base,
            url,
            main_frame_guid,
        })
    }

    /// Returns the channel for sending protocol messages
    ///
    /// Used internally for sending RPC calls to the page.
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Returns the main frame of the page.
    ///
    /// The main frame is where navigation and DOM operations actually happen.
    pub(crate) async fn main_frame(&self) -> Result<crate::protocol::Frame> {
        // Get the Frame object from the connection's object registry
        let frame_arc = self.connection().get_object(&self.main_frame_guid).await?;

        // Downcast to Frame
        let frame = frame_arc
            .as_any()
            .downcast_ref::<crate::protocol::Frame>()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(format!(
                    "Expected Frame object, got {}",
                    frame_arc.type_name()
                ))
            })?;

        Ok(frame.clone())
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
    pub fn url(&self) -> String {
        // Return a clone of the current URL
        self.url.read().unwrap().clone()
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

    /// Navigates to the specified URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to
    /// * `options` - Optional navigation options (timeout, wait_until)
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
    /// // Navigate to URL
    /// let response = page.goto("https://example.com", None).await?;
    /// assert!(response.ok);
    /// assert_eq!(page.url(), "https://example.com/");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - URL is invalid
    /// - Navigation timeout (default 30s)
    /// - Network error
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-goto>
    pub async fn goto(&self, url: &str, options: Option<GotoOptions>) -> Result<Response> {
        // Delegate to main frame
        let frame = self.main_frame().await?;
        let response = frame.goto(url, options).await?;

        // Update the page's URL
        if let Ok(mut page_url) = self.url.write() {
            *page_url = response.url().to_string();
        }

        Ok(response)
    }

    /// Returns the page's title.
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
    /// page.goto("https://example.com", None).await?;
    /// let title = page.title().await?;
    /// println!("Page title: {}", title);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-title>
    pub async fn title(&self) -> Result<String> {
        // Delegate to main frame
        let frame = self.main_frame().await?;
        frame.title().await
    }

    /// Creates a locator for finding elements on the page.
    ///
    /// Locators are the central piece of Playwright's auto-waiting and retry-ability.
    /// They don't execute queries until an action is performed.
    ///
    /// # Arguments
    ///
    /// * `selector` - CSS selector or other locating strategy
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
    /// page.goto("https://example.com", None).await?;
    ///
    /// // Create a locator
    /// let heading = page.locator("h1").await;
    ///
    /// // Get text content (locator executes now)
    /// let text = heading.text_content().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-locator>
    pub async fn locator(&self, selector: &str) -> crate::protocol::Locator {
        // Get the main frame
        let frame = self.main_frame().await.expect("Main frame should exist");

        crate::protocol::Locator::new(Arc::new(frame), selector.to_string())
    }

    /// Returns the keyboard instance for low-level keyboard control.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-keyboard>
    pub fn keyboard(&self) -> crate::protocol::Keyboard {
        crate::protocol::Keyboard::new(self.clone())
    }

    /// Returns the mouse instance for low-level mouse control.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-mouse>
    pub fn mouse(&self) -> crate::protocol::Mouse {
        crate::protocol::Mouse::new(self.clone())
    }

    // Internal keyboard methods (called by Keyboard struct)

    pub(crate) async fn keyboard_down(&self, key: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardDown",
                serde_json::json!({
                    "key": key
                }),
            )
            .await
    }

    pub(crate) async fn keyboard_up(&self, key: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardUp",
                serde_json::json!({
                    "key": key
                }),
            )
            .await
    }

    pub(crate) async fn keyboard_press(&self, key: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardPress",
                serde_json::json!({
                    "key": key
                }),
            )
            .await
    }

    pub(crate) async fn keyboard_type(&self, text: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardType",
                serde_json::json!({
                    "text": text
                }),
            )
            .await
    }

    pub(crate) async fn keyboard_insert_text(&self, text: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardInsertText",
                serde_json::json!({
                    "text": text
                }),
            )
            .await
    }

    // Internal mouse methods (called by Mouse struct)

    pub(crate) async fn mouse_move(&self, x: i32, y: i32) -> Result<()> {
        self.channel()
            .send_no_result(
                "mouseMove",
                serde_json::json!({
                    "x": x,
                    "y": y
                }),
            )
            .await
    }

    pub(crate) async fn mouse_click(&self, x: i32, y: i32) -> Result<()> {
        self.channel()
            .send_no_result(
                "mouseClick",
                serde_json::json!({
                    "x": x,
                    "y": y
                }),
            )
            .await
    }

    pub(crate) async fn mouse_dblclick(&self, x: i32, y: i32) -> Result<()> {
        self.channel()
            .send_no_result(
                "mouseClick",
                serde_json::json!({
                    "x": x,
                    "y": y,
                    "clickCount": 2
                }),
            )
            .await
    }

    pub(crate) async fn mouse_down(&self) -> Result<()> {
        self.channel()
            .send_no_result("mouseDown", serde_json::json!({}))
            .await
    }

    pub(crate) async fn mouse_up(&self) -> Result<()> {
        self.channel()
            .send_no_result("mouseUp", serde_json::json!({}))
            .await
    }

    pub(crate) async fn mouse_wheel(&self, delta_x: i32, delta_y: i32) -> Result<()> {
        self.channel()
            .send_no_result(
                "mouseWheel",
                serde_json::json!({
                    "deltaX": delta_x,
                    "deltaY": delta_y
                }),
            )
            .await
    }

    /// Reloads the current page.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional reload options (timeout, wait_until)
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
    /// page.goto("https://example.com", None).await?;
    /// let response = page.reload(None).await?;
    /// assert!(response.ok);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-reload>
    pub async fn reload(&self, options: Option<GotoOptions>) -> Result<Response> {
        // Build params
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            if let Some(timeout) = opts.timeout {
                params["timeout"] = serde_json::json!(timeout.as_millis() as u64);
            }
            if let Some(wait_until) = opts.wait_until {
                params["waitUntil"] = serde_json::json!(wait_until.as_str());
            }
        }

        // Send reload RPC directly to Page (not Frame!)
        #[derive(Deserialize)]
        struct ReloadResponse {
            response: Option<ResponseReference>,
        }

        #[derive(Deserialize)]
        struct ResponseReference {
            guid: String,
        }

        let reload_result: ReloadResponse = self.channel().send("reload", params).await?;

        // If reload returned a response, get the Response object
        if let Some(response_ref) = reload_result.response {
            // Wait for Response object to be created
            let response_arc = {
                let mut attempts = 0;
                let max_attempts = 20;
                loop {
                    match self.connection().get_object(&response_ref.guid).await {
                        Ok(obj) => break obj,
                        Err(_) if attempts < max_attempts => {
                            attempts += 1;
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        }
                        Err(e) => return Err(e),
                    }
                }
            };

            // Extract response data from initializer
            let initializer = response_arc.initializer();

            let status = initializer["status"].as_u64().ok_or_else(|| {
                crate::error::Error::ProtocolError("Response missing status".to_string())
            })? as u16;

            let headers = initializer["headers"]
                .as_array()
                .ok_or_else(|| {
                    crate::error::Error::ProtocolError("Response missing headers".to_string())
                })?
                .iter()
                .filter_map(|h| {
                    let name = h["name"].as_str()?;
                    let value = h["value"].as_str()?;
                    Some((name.to_string(), value.to_string()))
                })
                .collect();

            let response = Response {
                url: initializer["url"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::Error::ProtocolError("Response missing url".to_string())
                    })?
                    .to_string(),
                status,
                status_text: initializer["statusText"].as_str().unwrap_or("").to_string(),
                ok: (200..300).contains(&status),
                headers,
            };

            // Update the page's URL
            if let Ok(mut page_url) = self.url.write() {
                *page_url = response.url().to_string();
            }

            Ok(response)
        } else {
            Err(crate::error::Error::ProtocolError(
                "Reload did not return a response".to_string(),
            ))
        }
    }

    /// Takes a screenshot of the page and returns the image bytes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    ///
    /// page.goto("https://example.com", None).await?;
    ///
    /// // Capture screenshot as bytes
    /// let bytes = page.screenshot(None).await?;
    /// assert!(!bytes.is_empty());
    ///
    /// browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
    pub async fn screenshot(&self, _options: Option<()>) -> Result<Vec<u8>> {
        // For now, use default options - PNG format
        let params = serde_json::json!({
            "type": "png"
        });

        #[derive(Deserialize)]
        struct ScreenshotResponse {
            binary: String,
        }

        let response: ScreenshotResponse = self.channel().send("screenshot", params).await?;

        // Decode base64 to bytes
        let bytes = base64::prelude::BASE64_STANDARD
            .decode(&response.binary)
            .map_err(|e| {
                crate::error::Error::ProtocolError(format!("Failed to decode screenshot: {}", e))
            })?;

        Ok(bytes)
    }

    /// Takes a screenshot and saves it to a file, also returning the bytes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # use std::path::PathBuf;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    ///
    /// page.goto("https://example.com", None).await?;
    ///
    /// // Save screenshot to file
    /// let path = PathBuf::from("screenshot.png");
    /// let bytes = page.screenshot_to_file(&path, None).await?;
    /// assert!(path.exists());
    ///
    /// browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
    pub async fn screenshot_to_file(
        &self,
        path: &std::path::Path,
        options: Option<()>,
    ) -> Result<Vec<u8>> {
        // Get the screenshot bytes
        let bytes = self.screenshot(options).await?;

        // Write to file
        tokio::fs::write(path, &bytes).await.map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to write screenshot file: {}", e))
        })?;

        Ok(bytes)
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

    fn on_event(&self, method: &str, params: Value) {
        match method {
            "navigated" => {
                // Update URL when page navigates
                if let Some(url_value) = params.get("url") {
                    if let Some(url_str) = url_value.as_str() {
                        if let Ok(mut url) = self.url.write() {
                            *url = url_str.to_string();
                        }
                    }
                }
            }
            _ => {
                // Other events will be handled in future phases
                // Events: load, domcontentloaded, close, crash, etc.
            }
        }
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

/// Options for page.goto() and page.reload()
#[derive(Debug, Clone)]
pub struct GotoOptions {
    /// Maximum operation time in milliseconds. Default: 30000 (30 seconds)
    pub timeout: Option<std::time::Duration>,
    /// When to consider operation succeeded
    pub wait_until: Option<WaitUntil>,
}

impl GotoOptions {
    /// Creates new GotoOptions with default values
    pub fn new() -> Self {
        Self {
            timeout: None,
            wait_until: None,
        }
    }

    /// Sets the timeout
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the wait_until option
    pub fn wait_until(mut self, wait_until: WaitUntil) -> Self {
        self.wait_until = Some(wait_until);
        self
    }
}

impl Default for GotoOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// When to consider navigation succeeded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitUntil {
    /// Consider operation to be finished when the `load` event is fired
    Load,
    /// Consider operation to be finished when the `DOMContentLoaded` event is fired
    DomContentLoaded,
    /// Consider operation to be finished when there are no network connections for at least 500ms
    NetworkIdle,
    /// Consider operation to be finished when the commit event is fired
    Commit,
}

impl WaitUntil {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            WaitUntil::Load => "load",
            WaitUntil::DomContentLoaded => "domcontentloaded",
            WaitUntil::NetworkIdle => "networkidle",
            WaitUntil::Commit => "commit",
        }
    }
}

/// Response from navigation operations
#[derive(Debug, Clone)]
pub struct Response {
    /// URL of the response
    pub url: String,
    /// HTTP status code
    pub status: u16,
    /// HTTP status text
    pub status_text: String,
    /// Whether the response was successful (status 200-299)
    pub ok: bool,
    /// Response headers
    pub headers: std::collections::HashMap<String, String>,
}

impl Response {
    /// Returns the URL of the response
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the HTTP status code
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Returns the HTTP status text
    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    /// Returns whether the response was successful (status 200-299)
    pub fn ok(&self) -> bool {
        self.ok
    }

    /// Returns the response headers
    pub fn headers(&self) -> &std::collections::HashMap<String, String> {
        &self.headers
    }
}
