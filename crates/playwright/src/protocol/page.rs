// Page protocol object
//
// Represents a web page within a browser context.
// Pages are isolated tabs or windows within a context.

use crate::error::{Error, Result};
use crate::protocol::{Dialog, Download, Route, WebSocket};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use base64::Engine;
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};

/// Page represents a web page within a browser context.
///
/// A Page is created when you call `BrowserContext::new_page()` or `Browser::new_page()`.
/// Each page is an isolated tab/window within its parent context.
///
/// Initially, pages are navigated to "about:blank". Use navigation methods
/// Use navigation methods to navigate to URLs.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::{Playwright, ScreenshotOptions, ScreenshotType, AddStyleTagOptions, Viewport};
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let browser = playwright.chromium().launch().await?;
///     let page = browser.new_page().await?;
///
///     // Demonstrate url() - initially at about:blank
///     assert_eq!(page.url(), "about:blank");
///
///     // Demonstrate goto() - navigate to a page
///     let html = r#"<!DOCTYPE html>
///         <html>
///             <head><title>Test Page</title></head>
///             <body>
///                 <h1 id="heading">Hello World</h1>
///                 <p>First paragraph</p>
///                 <p>Second paragraph</p>
///                 <button onclick="alert('Alert!')">Alert</button>
///                 <a href="data:text/plain,file" download="test.txt">Download</a>
///             </body>
///         </html>
///     "#;
///     // Data URLs may not return a response (this is normal)
///     let _response = page.goto(&format!("data:text/html,{}", html), None).await?;
///
///     // Demonstrate title()
///     let title = page.title().await?;
///     assert_eq!(title, "Test Page");
///
///     // Demonstrate content() - returns full HTML including DOCTYPE
///     let content = page.content().await?;
///     assert!(content.contains("<!DOCTYPE html>") || content.to_lowercase().contains("<!doctype html>"));
///     assert!(content.contains("<title>Test Page</title>"));
///     assert!(content.contains("Hello World"));
///
///     // Demonstrate locator()
///     let heading = page.locator("#heading").await;
///     let text = heading.text_content().await?;
///     assert_eq!(text, Some("Hello World".to_string()));
///
///     // Demonstrate query_selector()
///     let element = page.query_selector("h1").await?;
///     assert!(element.is_some(), "Should find the h1 element");
///
///     // Demonstrate query_selector_all()
///     let paragraphs = page.query_selector_all("p").await?;
///     assert_eq!(paragraphs.len(), 2);
///
///     // Demonstrate evaluate()
///     page.evaluate::<(), ()>("console.log('Hello from Playwright!')", None).await?;
///
///     // Demonstrate evaluate_value()
///     let result = page.evaluate_value("1 + 1").await?;
///     assert_eq!(result, "2");
///
///     // Demonstrate screenshot()
///     let bytes = page.screenshot(None).await?;
///     assert!(!bytes.is_empty());
///
///     // Demonstrate screenshot_to_file()
///     let temp_dir = std::env::temp_dir();
///     let path = temp_dir.join("playwright_doctest_screenshot.png");
///     let bytes = page.screenshot_to_file(&path, Some(
///         ScreenshotOptions::builder()
///             .screenshot_type(ScreenshotType::Png)
///             .build()
///     )).await?;
///     assert!(!bytes.is_empty());
///
///     // Demonstrate reload()
///     // Data URLs may not return a response on reload (this is normal)
///     let _response = page.reload(None).await?;
///
///     // Demonstrate route() - network interception
///     page.route("**/*.png", |route| async move {
///         route.abort(None).await
///     }).await?;
///
///     // Demonstrate on_download() - download handler
///     page.on_download(|download| async move {
///         println!("Download started: {}", download.url());
///         Ok(())
///     }).await?;
///
///     // Demonstrate on_dialog() - dialog handler
///     page.on_dialog(|dialog| async move {
///         println!("Dialog: {} - {}", dialog.type_(), dialog.message());
///         dialog.accept(None).await
///     }).await?;
///
///     // Demonstrate add_style_tag() - inject CSS
///     page.add_style_tag(
///         AddStyleTagOptions::builder()
///             .content("body { background-color: blue; }")
///             .build()
///     ).await?;
///
///     // Demonstrate set_viewport_size() - responsive testing
///     let mobile_viewport = Viewport {
///         width: 375,
///         height: 667,
///     };
///     page.set_viewport_size(mobile_viewport).await?;
///
///     // Demonstrate close()
///     page.close().await?;
///
///     browser.close().await?;
///     Ok(())
/// }
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
    main_frame_guid: Arc<str>,
    /// Cached reference to the main frame for synchronous URL access
    /// This is populated after the first call to main_frame()
    cached_main_frame: Arc<Mutex<Option<crate::protocol::Frame>>>,
    /// Route handlers for network interception
    route_handlers: Arc<Mutex<Vec<RouteHandlerEntry>>>,
    /// Download event handlers
    download_handlers: Arc<Mutex<Vec<DownloadHandler>>>,
    /// Dialog event handlers
    dialog_handlers: Arc<Mutex<Vec<DialogHandler>>>,
    /// WebSocket event handlers
    websocket_handlers: Arc<Mutex<Vec<WebSocketHandler>>>,
}

/// Type alias for boxed route handler future
type RouteHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed download handler future
type DownloadHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed dialog handler future
type DialogHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed websocket handler future
type WebSocketHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Storage for a single route handler
#[derive(Clone)]
struct RouteHandlerEntry {
    pattern: String,
    handler: Arc<dyn Fn(Route) -> RouteHandlerFuture + Send + Sync>,
}

/// Download event handler
type DownloadHandler = Arc<dyn Fn(Download) -> DownloadHandlerFuture + Send + Sync>;

/// Dialog event handler
type DialogHandler = Arc<dyn Fn(Dialog) -> DialogHandlerFuture + Send + Sync>;

/// WebSocket event handler
type WebSocketHandler = Arc<dyn Fn(WebSocket) -> WebSocketHandlerFuture + Send + Sync>;

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
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        // Extract mainFrame GUID from initializer
        let main_frame_guid: Arc<str> =
            Arc::from(initializer["mainFrame"]["guid"].as_str().ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "Page initializer missing 'mainFrame.guid' field".to_string(),
                )
            })?);

        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer,
        );

        // Initialize URL to about:blank
        let url = Arc::new(RwLock::new("about:blank".to_string()));

        // Initialize empty route handlers
        let route_handlers = Arc::new(Mutex::new(Vec::new()));

        // Initialize empty event handlers
        let download_handlers = Arc::new(Mutex::new(Vec::new()));
        let dialog_handlers = Arc::new(Mutex::new(Vec::new()));
        let websocket_handlers = Arc::new(Mutex::new(Vec::new()));

        // Initialize cached main frame as empty (will be populated on first access)
        let cached_main_frame = Arc::new(Mutex::new(None));

        Ok(Self {
            base,
            url,
            main_frame_guid,
            cached_main_frame,
            route_handlers,
            download_handlers,
            dialog_handlers,
            websocket_handlers,
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
    pub async fn main_frame(&self) -> Result<crate::protocol::Frame> {
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

        let frame_clone = frame.clone();

        // Cache the frame for synchronous access in url()
        if let Ok(mut cached) = self.cached_main_frame.lock() {
            *cached = Some(frame_clone.clone());
        }

        Ok(frame_clone)
    }

    /// Returns the current URL of the page.
    ///
    /// This returns the last committed URL, including hash fragments from anchor navigation.
    /// Initially, pages are at "about:blank".
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-url>
    pub fn url(&self) -> String {
        // Try to get URL from the cached main frame (source of truth for navigation including hashes)
        if let Ok(cached) = self.cached_main_frame.lock() {
            if let Some(frame) = cached.as_ref() {
                return frame.url();
            }
        }

        // Fallback to cached URL if frame not yet loaded
        self.url.read().unwrap().clone()
    }

    /// Closes the page.
    ///
    /// This is a graceful operation that sends a close command to the page
    /// and waits for it to shut down properly.
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
    /// Returns `None` when navigating to URLs that don't produce responses (e.g., data URLs,
    /// about:blank). This matches Playwright's behavior across all language bindings.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to
    /// * `options` - Optional navigation options (timeout, wait_until)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - URL is invalid
    /// - Navigation timeout (default 30s)
    /// - Network error
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-goto>
    pub async fn goto(&self, url: &str, options: Option<GotoOptions>) -> Result<Option<Response>> {
        // Delegate to main frame
        let frame = self.main_frame().await.map_err(|e| match e {
            Error::TargetClosed { context, .. } => Error::TargetClosed {
                target_type: "Page".to_string(),
                context,
            },
            other => other,
        })?;

        let response = frame.goto(url, options).await.map_err(|e| match e {
            Error::TargetClosed { context, .. } => Error::TargetClosed {
                target_type: "Page".to_string(),
                context,
            },
            other => other,
        })?;

        // Update the page's URL if we got a response
        if let Some(ref resp) = response {
            if let Ok(mut page_url) = self.url.write() {
                *page_url = resp.url().to_string();
            }
        }

        Ok(response)
    }

    /// Returns the browser context that the page belongs to.
    pub fn context(&self) -> Result<crate::protocol::BrowserContext> {
        let parent = self.base.parent().ok_or_else(|| Error::TargetClosed {
            target_type: "Page".into(),
            context: "Parent context not found".into(),
        })?;

        let context = parent
            .as_any()
            .downcast_ref::<crate::protocol::BrowserContext>()
            .ok_or_else(|| {
                Error::ProtocolError("Page parent is not a BrowserContext".to_string())
            })?;

        Ok(context.clone())
    }

    /// Pauses script execution.
    ///
    /// Playwright will stop executing the script and wait for the user to either press
    /// "Resume" in the page overlay or in the debugger.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-pause>
    pub async fn pause(&self) -> Result<()> {
        self.context()?.pause().await
    }

    /// Returns the page's title.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-title>
    pub async fn title(&self) -> Result<String> {
        // Delegate to main frame
        let frame = self.main_frame().await?;
        frame.title().await
    }

    /// Returns the full HTML content of the page, including the DOCTYPE.
    ///
    /// This method retrieves the complete HTML markup of the page,
    /// including the doctype declaration and all DOM elements.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-content>
    pub async fn content(&self) -> Result<String> {
        // Delegate to main frame
        let frame = self.main_frame().await?;
        frame.content().await
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

    pub(crate) async fn keyboard_press(
        &self,
        key: &str,
        options: Option<crate::protocol::KeyboardOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "key": key
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        }

        self.channel().send_no_result("keyboardPress", params).await
    }

    pub(crate) async fn keyboard_type(
        &self,
        text: &str,
        options: Option<crate::protocol::KeyboardOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "text": text
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        }

        self.channel().send_no_result("keyboardType", params).await
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

    pub(crate) async fn mouse_move(
        &self,
        x: i32,
        y: i32,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "x": x,
            "y": y
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        }

        self.channel().send_no_result("mouseMove", params).await
    }

    pub(crate) async fn mouse_click(
        &self,
        x: i32,
        y: i32,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "x": x,
            "y": y
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        }

        self.channel().send_no_result("mouseClick", params).await
    }

    pub(crate) async fn mouse_dblclick(
        &self,
        x: i32,
        y: i32,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "x": x,
            "y": y,
            "clickCount": 2
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        }

        self.channel().send_no_result("mouseClick", params).await
    }

    pub(crate) async fn mouse_down(
        &self,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        }

        self.channel().send_no_result("mouseDown", params).await
    }

    pub(crate) async fn mouse_up(
        &self,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        }

        self.channel().send_no_result("mouseUp", params).await
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
    /// Returns `None` when reloading pages that don't produce responses (e.g., data URLs,
    /// about:blank). This matches Playwright's behavior across all language bindings.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-reload>
    pub async fn reload(&self, options: Option<GotoOptions>) -> Result<Option<Response>> {
        // Build params
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            if let Some(timeout) = opts.timeout {
                params["timeout"] = serde_json::json!(timeout.as_millis() as u64);
            } else {
                params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
            }
            if let Some(wait_until) = opts.wait_until {
                params["waitUntil"] = serde_json::json!(wait_until.as_str());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        // Send reload RPC directly to Page (not Frame!)
        #[derive(Deserialize)]
        struct ReloadResponse {
            response: Option<ResponseReference>,
        }

        #[derive(Deserialize)]
        struct ResponseReference {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
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

            Ok(Some(response))
        } else {
            // Reload returned null (e.g., data URLs, about:blank)
            // This is a valid result, not an error
            Ok(None)
        }
    }

    /// Returns the first element matching the selector, or None if not found.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-query-selector>
    pub async fn query_selector(
        &self,
        selector: &str,
    ) -> Result<Option<Arc<crate::protocol::ElementHandle>>> {
        let frame = self.main_frame().await?;
        frame.query_selector(selector).await
    }

    /// Returns all elements matching the selector.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-query-selector-all>
    pub async fn query_selector_all(
        &self,
        selector: &str,
    ) -> Result<Vec<Arc<crate::protocol::ElementHandle>>> {
        let frame = self.main_frame().await?;
        frame.query_selector_all(selector).await
    }

    /// Takes a screenshot of the page and returns the image bytes.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
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
    /// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
    pub async fn screenshot_to_file(
        &self,
        path: &std::path::Path,
        options: Option<crate::protocol::ScreenshotOptions>,
    ) -> Result<Vec<u8>> {
        // Get the screenshot bytes
        let bytes = self.screenshot(options).await?;

        // Write to file
        tokio::fs::write(path, &bytes).await.map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to write screenshot file: {}", e))
        })?;

        Ok(bytes)
    }

    /// Evaluates JavaScript in the page context (without return value).
    ///
    /// Executes the provided JavaScript expression or function within the page's
    /// context without returning a value.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    pub async fn evaluate_expression(&self, expression: &str) -> Result<()> {
        // Delegate to the main frame
        let frame = self.main_frame().await?;
        frame.frame_evaluate_expression(expression).await
    }

    /// Evaluates JavaScript in the page context with optional arguments.
    ///
    /// Executes the provided JavaScript expression or function within the page's
    /// context and returns the result. The return value must be JSON-serializable.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript code to evaluate
    /// * `arg` - Optional argument to pass to the expression (must implement Serialize)
    ///
    /// # Returns
    ///
    /// The result as a `serde_json::Value`
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    pub async fn evaluate<T: serde::Serialize, U: serde::de::DeserializeOwned>(
        &self,
        expression: &str,
        arg: Option<&T>,
    ) -> Result<U> {
        // Delegate to the main frame
        let frame = self.main_frame().await?;
        let result = frame.evaluate(expression, arg).await?;
        serde_json::from_value(result).map_err(Error::from)
    }

    /// Evaluates a JavaScript expression and returns the result as a String.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript code to evaluate
    ///
    /// # Returns
    ///
    /// The result converted to a String
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    pub async fn evaluate_value(&self, expression: &str) -> Result<String> {
        let frame = self.main_frame().await?;
        frame.frame_evaluate_expression_value(expression).await
    }

    /// Registers a route handler for network interception.
    ///
    /// When a request matches the specified pattern, the handler will be called
    /// with a Route object that can abort, continue, or fulfill the request.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to match (supports glob patterns like "**/*.png")
    /// * `handler` - Async closure that handles the route
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-route>
    pub async fn route<F, Fut>(&self, pattern: &str, handler: F) -> Result<()>
    where
        F: Fn(Route) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        // 1. Wrap handler in Arc with type erasure
        let handler =
            Arc::new(move |route: Route| -> RouteHandlerFuture { Box::pin(handler(route)) });

        // 2. Store in handlers list
        self.route_handlers.lock().unwrap().push(RouteHandlerEntry {
            pattern: pattern.to_string(),
            handler,
        });

        // 3. Enable network interception via protocol
        self.enable_network_interception().await?;

        Ok(())
    }

    /// Updates network interception patterns for this page
    async fn enable_network_interception(&self) -> Result<()> {
        // Collect all patterns from registered handlers
        // Each pattern must be an object with "glob" field
        let patterns: Vec<serde_json::Value> = self
            .route_handlers
            .lock()
            .unwrap()
            .iter()
            .map(|entry| serde_json::json!({ "glob": entry.pattern }))
            .collect();

        // Send protocol command to update network interception patterns
        // Follows playwright-python's approach
        self.channel()
            .send_no_result(
                "setNetworkInterceptionPatterns",
                serde_json::json!({
                    "patterns": patterns
                }),
            )
            .await
    }

    /// Handles a route event from the protocol
    ///
    /// Called by on_event when a "route" event is received
    async fn on_route_event(&self, route: Route) {
        let handlers = self.route_handlers.lock().unwrap().clone();
        let url = route.request().url().to_string();

        // Find matching handler (last registered wins)
        for entry in handlers.iter().rev() {
            // Use glob pattern matching
            if Self::matches_pattern(&entry.pattern, &url) {
                let handler = entry.handler.clone();
                // Execute handler and wait for completion
                // This ensures fulfill/continue/abort completes before browser continues
                if let Err(e) = handler(route).await {
                    tracing::warn!("Route handler error: {}", e);
                }
                break;
            }
        }
    }

    /// Checks if a URL matches a glob pattern
    ///
    /// Supports standard glob patterns:
    /// - `*` matches any characters except `/`
    /// - `**` matches any characters including `/`
    /// - `?` matches a single character
    fn matches_pattern(pattern: &str, url: &str) -> bool {
        use glob::Pattern;

        // Try to compile the glob pattern
        match Pattern::new(pattern) {
            Ok(glob_pattern) => glob_pattern.matches(url),
            Err(_) => {
                // If pattern is invalid, fall back to exact string match
                pattern == url
            }
        }
    }

    /// Registers a download event handler.
    ///
    /// The handler will be called when a download is triggered by the page.
    /// Downloads occur when the page initiates a file download (e.g., clicking a link
    /// with the download attribute, or a server response with Content-Disposition: attachment).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the Download object
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-download>
    pub async fn on_download<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Download) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        // Wrap handler with type erasure
        let handler = Arc::new(move |download: Download| -> DownloadHandlerFuture {
            Box::pin(handler(download))
        });

        // Store handler
        self.download_handlers.lock().unwrap().push(handler);

        Ok(())
    }

    /// Registers a dialog event handler.
    ///
    /// The handler will be called when a JavaScript dialog is triggered (alert, confirm, prompt, or beforeunload).
    /// The dialog must be explicitly accepted or dismissed, otherwise the page will freeze.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the Dialog object
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-dialog>
    pub async fn on_dialog<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Dialog) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        // Wrap handler with type erasure
        let handler =
            Arc::new(move |dialog: Dialog| -> DialogHandlerFuture { Box::pin(handler(dialog)) });

        // Store handler
        self.dialog_handlers.lock().unwrap().push(handler);

        // Dialog events are auto-emitted (no subscription needed)

        Ok(())
    }

    /// Adds a listener for the `websocket` event.
    ///
    /// The handler will be called when a WebSocket request is dispatched.
    ///
    /// # Arguments
    ///
    /// * `handler` - The function to call when the event occurs
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-on-websocket>
    pub async fn on_websocket<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(WebSocket) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler =
            Arc::new(move |ws: WebSocket| -> WebSocketHandlerFuture { Box::pin(handler(ws)) });
        self.websocket_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Handles a download event from the protocol
    async fn on_download_event(&self, download: Download) {
        let handlers = self.download_handlers.lock().unwrap().clone();

        for handler in handlers {
            if let Err(e) = handler(download.clone()).await {
                tracing::warn!("Download handler error: {}", e);
            }
        }
    }

    /// Handles a dialog event from the protocol
    async fn on_dialog_event(&self, dialog: Dialog) {
        let handlers = self.dialog_handlers.lock().unwrap().clone();

        for handler in handlers {
            if let Err(e) = handler(dialog.clone()).await {
                tracing::warn!("Dialog handler error: {}", e);
            }
        }
    }

    /// Triggers dialog event (called by BrowserContext when dialog events arrive)
    ///
    /// Dialog events are sent to BrowserContext and forwarded to the associated Page.
    /// This method is public so BrowserContext can forward dialog events.
    pub async fn trigger_dialog_event(&self, dialog: Dialog) {
        self.on_dialog_event(dialog).await;
    }

    /// Adds a `<style>` tag into the page with the desired content.
    ///
    /// # Arguments
    ///
    /// * `options` - Style tag options (content, url, or path)
    ///
    /// # Returns
    ///
    /// Returns an ElementHandle pointing to the injected `<style>` tag
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, AddStyleTagOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let context = browser.new_context().await?;
    /// # let page = context.new_page().await?;
    /// use playwright_rs::protocol::AddStyleTagOptions;
    ///
    /// // With inline CSS
    /// page.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .content("body { background-color: red; }")
    ///         .build()
    /// ).await?;
    ///
    /// // With external URL
    /// page.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .url("https://example.com/style.css")
    ///         .build()
    /// ).await?;
    ///
    /// // From file
    /// page.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .path("./styles/custom.css")
    ///         .build()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-add-style-tag>
    pub async fn add_style_tag(
        &self,
        options: AddStyleTagOptions,
    ) -> Result<Arc<crate::protocol::ElementHandle>> {
        let frame = self.main_frame().await?;
        frame.add_style_tag(options).await
    }

    /// Adds a script which would be evaluated in one of the following scenarios:
    /// - Whenever the page is navigated
    /// - Whenever a child frame is attached or navigated
    ///
    /// The script is evaluated after the document was created but before any of its scripts were run.
    ///
    /// # Arguments
    ///
    /// * `script` - JavaScript code to be injected into the page
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let context = browser.new_context().await?;
    /// # let page = context.new_page().await?;
    /// page.add_init_script("window.injected = 123;").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-add-init-script>
    pub async fn add_init_script(&self, script: &str) -> Result<()> {
        self.channel()
            .send_no_result("addInitScript", serde_json::json!({ "source": script }))
            .await
    }

    /// Sets the viewport size for the page.
    ///
    /// This method allows dynamic resizing of the viewport after page creation,
    /// useful for testing responsive layouts at different screen sizes.
    ///
    /// # Arguments
    ///
    /// * `viewport` - The viewport dimensions (width and height in pixels)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, Viewport};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// // Set viewport to mobile size
    /// let mobile = Viewport {
    ///     width: 375,
    ///     height: 667,
    /// };
    /// page.set_viewport_size(mobile).await?;
    ///
    /// // Later, test desktop layout
    /// let desktop = Viewport {
    ///     width: 1920,
    ///     height: 1080,
    /// };
    /// page.set_viewport_size(desktop).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-viewport-size>
    pub async fn set_viewport_size(&self, viewport: crate::protocol::Viewport) -> Result<()> {
        self.channel()
            .send_no_result(
                "setViewportSize",
                serde_json::json!({ "viewportSize": viewport }),
            )
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
            "route" => {
                // Handle network routing event
                if let Some(route_guid) = params
                    .get("route")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    // Get the Route object from connection's registry
                    let connection = self.connection();
                    let route_guid_owned = route_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        // Wait for Route object to be created
                        let route_arc = match connection.get_object(&route_guid_owned).await {
                            Ok(obj) => obj,
                            Err(e) => {
                                tracing::warn!("Failed to get route object: {}", e);
                                return;
                            }
                        };

                        // Downcast to Route
                        let route = match route_arc.as_any().downcast_ref::<Route>() {
                            Some(r) => r.clone(),
                            None => {
                                tracing::warn!("Failed to downcast to Route");
                                return;
                            }
                        };

                        // Call the route handler and wait for completion
                        self_clone.on_route_event(route).await;
                    });
                }
            }
            "download" => {
                // Handle download event
                // Event params: {url, suggestedFilename, artifact: {guid: "..."}}
                let url = params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let suggested_filename = params
                    .get("suggestedFilename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(artifact_guid) = params
                    .get("artifact")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let artifact_guid_owned = artifact_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        // Wait for Artifact object to be created
                        let artifact_arc = match connection.get_object(&artifact_guid_owned).await {
                            Ok(obj) => obj,
                            Err(e) => {
                                tracing::warn!("Failed to get artifact object: {}", e);
                                return;
                            }
                        };

                        // Create Download wrapper from Artifact + event params
                        let download =
                            Download::from_artifact(artifact_arc, url, suggested_filename);

                        // Call the download handlers
                        self_clone.on_download_event(download).await;
                    });
                }
            }
            "dialog" => {
                // Dialog events are handled by BrowserContext and forwarded to Page
                // This case should not be reached, but keeping for completeness
            }
            "webSocket" => {
                if let Some(ws_guid) = params
                    .get("webSocket")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let ws_guid_owned = ws_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        // Wait for WebSocket object to be created
                        let ws_arc = match connection.get_object(&ws_guid_owned).await {
                            Ok(obj) => obj,
                            Err(e) => {
                                tracing::warn!("Failed to get WebSocket object: {}", e);
                                return;
                            }
                        };

                        // Downcast to WebSocket
                        let ws = if let Some(ws) = ws_arc.as_any().downcast_ref::<WebSocket>() {
                            ws.clone()
                        } else {
                            tracing::warn!("Expected WebSocket object, got {}", ws_arc.type_name());
                            return;
                        };

                        // Call handlers
                        let handlers = self_clone.websocket_handlers.lock().unwrap().clone();
                        for handler in handlers {
                            let ws_clone = ws.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handler(ws_clone).await {
                                    tracing::error!("Error in websocket handler: {}", e);
                                }
                            });
                        }
                    });
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
    /// Maximum operation time in milliseconds
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

/// Options for adding a style tag to the page
///
/// See: <https://playwright.dev/docs/api/class-page#page-add-style-tag>
#[derive(Debug, Clone, Default)]
pub struct AddStyleTagOptions {
    /// Raw CSS content to inject
    pub content: Option<String>,
    /// URL of the `<link>` tag to add
    pub url: Option<String>,
    /// Path to a CSS file to inject
    pub path: Option<String>,
}

impl AddStyleTagOptions {
    /// Creates a new builder for AddStyleTagOptions
    pub fn builder() -> AddStyleTagOptionsBuilder {
        AddStyleTagOptionsBuilder::default()
    }

    /// Validates that at least one option is specified
    pub(crate) fn validate(&self) -> Result<()> {
        if self.content.is_none() && self.url.is_none() && self.path.is_none() {
            return Err(Error::InvalidArgument(
                "At least one of content, url, or path must be specified".to_string(),
            ));
        }
        Ok(())
    }
}

/// Builder for AddStyleTagOptions
#[derive(Debug, Clone, Default)]
pub struct AddStyleTagOptionsBuilder {
    content: Option<String>,
    url: Option<String>,
    path: Option<String>,
}

impl AddStyleTagOptionsBuilder {
    /// Sets the CSS content to inject
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Sets the URL of the stylesheet
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the path to a CSS file
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Builds the AddStyleTagOptions
    pub fn build(self) -> AddStyleTagOptions {
        AddStyleTagOptions {
            content: self.content,
            url: self.url,
            path: self.path,
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
