// Frame protocol object
//
// Represents a frame within a page. Pages have a main frame, and can have child frames (iframes).
// Navigation and DOM operations happen on frames, not directly on pages.

use crate::error::{Error, Result};
use crate::protocol::page::{GotoOptions, Response, WaitUntil};
use crate::protocol::{parse_result, serialize_argument, serialize_null};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::ConnectionExt;
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::sync::{Arc, Mutex, RwLock};

/// Frame represents a frame within a page.
///
/// Every page has a main frame, and pages can have additional child frames (iframes).
/// Frame is where navigation, selector queries, and DOM operations actually happen.
///
/// In Playwright's architecture, Page delegates navigation and interaction methods to Frame.
///
/// See: <https://playwright.dev/docs/api/class-frame>
#[derive(Clone)]
pub struct Frame {
    base: ChannelOwnerImpl,
    /// Current URL of the frame.
    /// Wrapped in RwLock to allow updates from events.
    url: Arc<RwLock<String>>,
    /// The name attribute of the frame element (empty string for the main frame).
    /// Extracted from the protocol initializer.
    name: Arc<str>,
    /// GUID of the parent frame, if any (None for the main/top-level frame).
    /// Extracted from the protocol initializer.
    parent_frame_guid: Option<Arc<str>>,
    /// Whether this frame has been detached from the page.
    /// Set to true when a "detached" event is received.
    is_detached: Arc<RwLock<bool>>,
    /// The owning Page, set after the Page is created and the frame is adopted.
    ///
    /// This is `None` until `set_page()` is called by the owning Page.
    /// Using `Mutex<Option<...>>` so that `set_page()` can be called on a shared `&Frame`.
    page: Arc<Mutex<Option<crate::protocol::Page>>>,
}

impl Frame {
    /// Creates a new Frame from protocol initialization.
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Frame object.
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

        // Extract initial URL from initializer if available
        let initial_url = initializer
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("about:blank")
            .to_string();

        let url = Arc::new(RwLock::new(initial_url));

        // Extract the frame's name attribute (empty string for main frame)
        let name: Arc<str> = Arc::from(
            initializer
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        );

        // Extract parent frame GUID if present
        let parent_frame_guid: Option<Arc<str>> = initializer
            .get("parentFrame")
            .and_then(|v| v.get("guid"))
            .and_then(|v| v.as_str())
            .map(Arc::from);

        Ok(Self {
            base,
            url,
            name,
            parent_frame_guid,
            is_detached: Arc::new(RwLock::new(false)),
            page: Arc::new(Mutex::new(None)),
        })
    }

    /// Sets the owning Page for this frame.
    ///
    /// Called by `Page::main_frame()` after the frame is retrieved from the registry.
    /// This allows `frame.page()` and `frame.locator()` to work.
    pub(crate) fn set_page(&self, page: crate::protocol::Page) {
        if let Ok(mut guard) = self.page.lock() {
            *guard = Some(page);
        }
    }

    /// Returns the owning Page for this frame, if it has been set.
    ///
    /// Returns `None` if `set_page()` has not been called yet (i.e., before the frame
    /// has been adopted by a Page). In normal usage the main frame always has a Page.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-page>
    pub fn page(&self) -> Option<crate::protocol::Page> {
        self.page.lock().ok().and_then(|g| g.clone())
    }

    /// Returns the `name` attribute value of the frame element used to create this frame.
    ///
    /// For the main (top-level) frame this is always an empty string.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-name>
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the parent `Frame`, or `None` if this is the top-level (main) frame.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-parent-frame>
    pub fn parent_frame(&self) -> Option<crate::protocol::Frame> {
        let guid = self.parent_frame_guid.as_ref()?;
        // Look up the parent frame in the connection registry (sync-compatible via block_on)
        // We spawn a brief async lookup using the connection.
        let conn = self.base.connection();
        // Use tokio's block_in_place / futures executor to do a synchronous resolution.
        // This mirrors how other Rust Playwright clients resolve parent references.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(conn.get_typed::<crate::protocol::Frame>(guid))
                .ok()
        })
    }

    /// Returns `true` if the frame has been detached from its page.
    ///
    /// A frame becomes detached when the corresponding `<iframe>` element is removed
    /// from the DOM or when the owning page is closed.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-is-detached>
    pub fn is_detached(&self) -> bool {
        self.is_detached.read().map(|v| *v).unwrap_or(false)
    }

    /// Returns all child frames embedded in this frame.
    ///
    /// Child frames are created by `<iframe>` elements within this frame.
    /// For the main frame this may include multiple iframes.
    ///
    /// # Implementation Note
    ///
    /// This iterates all objects in the connection registry to find `Frame` objects
    /// whose `parentFrame` initializer field matches this frame's GUID. This matches
    /// the relationship Playwright establishes when creating child frames.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-child-frames>
    pub fn child_frames(&self) -> Vec<crate::protocol::Frame> {
        let my_guid = self.guid().to_string();
        let conn = self.base.connection();

        // Use the synchronous registry snapshot — no async needed since the
        // underlying storage is a parking_lot::Mutex (sync-safe to lock).
        conn.all_objects_sync()
            .into_iter()
            .filter_map(|obj| {
                // Only consider Frame-typed objects
                if obj.type_name() != "Frame" {
                    return None;
                }
                // Check the initializer's parentFrame.guid field
                let parent_guid = obj
                    .initializer()
                    .get("parentFrame")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())?;

                if parent_guid == my_guid {
                    obj.as_any()
                        .downcast_ref::<crate::protocol::Frame>()
                        .cloned()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Evaluates a JavaScript expression and returns a handle to the result.
    ///
    /// Unlike [`evaluate`](Frame::evaluate) which serializes the return value to JSON,
    /// `evaluate_handle` returns a handle to the in-browser object. This is useful when
    /// the return value is a non-serializable DOM element or complex JS object.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript expression to evaluate in the frame context
    ///
    /// # Returns
    ///
    /// An `Arc<ElementHandle>` pointing to the in-browser object.
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
    /// let frame = page.main_frame().await?;
    ///
    /// let handle = frame.evaluate_handle("document.body").await?;
    /// let screenshot = handle.screenshot(None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The JavaScript expression throws an error
    /// - The result handle GUID cannot be found in the registry
    /// - Communication with the browser fails
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-evaluate-handle>
    pub async fn evaluate_handle(
        &self,
        expression: &str,
    ) -> Result<Arc<crate::protocol::ElementHandle>> {
        let params = serde_json::json!({
            "expression": expression,
            "isFunction": false,
            "arg": {"value": {"v": "undefined"}, "handles": []}
        });

        // The server returns {"handle": {"guid": "JSHandle@..."}}
        #[derive(Deserialize)]
        struct HandleRef {
            guid: String,
        }
        #[derive(Deserialize)]
        struct EvaluateHandleResponse {
            handle: HandleRef,
        }

        let response: EvaluateHandleResponse = self
            .channel()
            .send("evaluateExpressionHandle", params)
            .await?;

        let guid = &response.handle.guid;

        // Look up in the connection registry with retry (the __create__ may arrive slightly later)
        let connection = self.base.connection();
        let mut attempts = 0;
        let max_attempts = 20;
        let handle = loop {
            match connection
                .get_typed::<crate::protocol::ElementHandle>(guid)
                .await
            {
                Ok(h) => break h,
                Err(_) if attempts < max_attempts => {
                    attempts += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        };

        Ok(Arc::new(handle))
    }

    /// Evaluates a JavaScript expression and returns a [`JSHandle`](crate::protocol::JSHandle) to the result.
    ///
    /// Unlike [`evaluate_handle`](Frame::evaluate_handle) which returns an `Arc<ElementHandle>`,
    /// this method returns an `Arc<JSHandle>` and is suitable for non-DOM values such as
    /// plain objects, numbers, and strings.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript expression to evaluate in the frame context
    ///
    /// # Returns
    ///
    /// An `Arc<JSHandle>` pointing to the in-browser value.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The JavaScript expression throws an error
    /// - The result handle GUID cannot be found in the registry
    /// - Communication with the browser fails
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-evaluate-handle>
    pub async fn evaluate_handle_js(
        &self,
        expression: &str,
    ) -> Result<std::sync::Arc<crate::protocol::JSHandle>> {
        // Detect whether the expression is a function (arrow function or function keyword)
        // so we can set isFunction correctly and the server invokes it rather than
        // evaluating the function literal.
        let trimmed = expression.trim();
        let is_function = trimmed.starts_with("(")
            || trimmed.starts_with("function")
            || trimmed.starts_with("async ");

        let params = serde_json::json!({
            "expression": expression,
            "isFunction": is_function,
            "arg": {"value": {"v": "undefined"}, "handles": []}
        });

        // The server returns {"handle": {"guid": "JSHandle@..."}}
        #[derive(Deserialize)]
        struct HandleRef {
            guid: String,
        }
        #[derive(Deserialize)]
        struct EvaluateHandleResponse {
            handle: HandleRef,
        }

        let response: EvaluateHandleResponse = self
            .channel()
            .send("evaluateExpressionHandle", params)
            .await?;

        let guid = &response.handle.guid;

        // Look up in the connection registry with retry (the __create__ may arrive slightly later)
        let connection = self.base.connection();
        let mut attempts = 0;
        let max_attempts = 20;
        let handle = loop {
            match connection
                .get_typed::<crate::protocol::JSHandle>(guid)
                .await
            {
                Ok(h) => break h,
                Err(_) if attempts < max_attempts => {
                    attempts += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        };

        Ok(std::sync::Arc::new(handle))
    }

    /// Creates a [`Locator`](crate::protocol::Locator) scoped to this frame.
    ///
    /// The locator is lazy — it does not query the DOM until an action is performed on it.
    ///
    /// # Arguments
    ///
    /// * `selector` - A CSS selector or other Playwright selector strategy
    ///
    /// # Panics
    ///
    /// Panics if the owning Page has not been set (i.e., `set_page()` was never called).
    /// In normal usage the main frame always has its page wired up by `Page::main_frame()`.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-locator>
    pub fn locator(&self, selector: &str) -> crate::protocol::Locator {
        let page = self
            .page()
            .expect("Frame::locator() called before set_page(); call page.main_frame() first");
        crate::protocol::Locator::new(Arc::new(self.clone()), selector.to_string(), page)
    }

    /// Returns a locator that matches elements containing the given text.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-get-by-text>
    pub fn get_by_text(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_text_selector(text, exact))
    }

    /// Returns a locator that matches elements by their associated label text.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-get-by-label>
    pub fn get_by_label(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_label_selector(
            text, exact,
        ))
    }

    /// Returns a locator that matches elements by their placeholder text.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-get-by-placeholder>
    pub fn get_by_placeholder(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_placeholder_selector(
            text, exact,
        ))
    }

    /// Returns a locator that matches elements by their alt text.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-get-by-alt-text>
    pub fn get_by_alt_text(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_alt_text_selector(
            text, exact,
        ))
    }

    /// Returns a locator that matches elements by their title attribute.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-get-by-title>
    pub fn get_by_title(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_title_selector(
            text, exact,
        ))
    }

    /// Returns a locator that matches elements by their test ID attribute.
    ///
    /// By default, uses the `data-testid` attribute. Call
    /// `playwright.selectors().set_test_id_attribute()` to change the attribute name.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-get-by-test-id>
    pub fn get_by_test_id(&self, test_id: &str) -> crate::protocol::Locator {
        use crate::server::channel_owner::ChannelOwner;
        let attr = self.connection().selectors().test_id_attribute();
        self.locator(&crate::protocol::locator::get_by_test_id_selector_with_attr(test_id, &attr))
    }

    /// Returns a locator that matches elements by their ARIA role.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-get-by-role>
    pub fn get_by_role(
        &self,
        role: crate::protocol::locator::AriaRole,
        options: Option<crate::protocol::locator::GetByRoleOptions>,
    ) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_role_selector(
            role, options,
        ))
    }

    /// Returns the channel for sending protocol messages
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Returns the current URL of the frame.
    ///
    /// This returns the last committed URL. Initially, frames are at "about:blank".
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-url>
    pub fn url(&self) -> String {
        self.url.read().unwrap().clone()
    }

    /// Navigates the frame to the specified URL.
    ///
    /// This is the actual protocol method for navigation. Page.goto() delegates to this.
    ///
    /// Returns `None` when navigating to URLs that don't produce responses (e.g., data URLs,
    /// about:blank). This matches Playwright's behavior across all language bindings.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to
    /// * `options` - Optional navigation options (timeout, wait_until)
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-goto>
    pub async fn goto(&self, url: &str, options: Option<GotoOptions>) -> Result<Option<Response>> {
        // Build params manually using json! macro
        let mut params = serde_json::json!({
            "url": url,
        });

        // Add optional parameters
        if let Some(opts) = options {
            if let Some(timeout) = opts.timeout {
                params["timeout"] = serde_json::json!(timeout.as_millis() as u64);
            } else {
                // Default timeout required in Playwright 1.56.1+
                params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
            }
            if let Some(wait_until) = opts.wait_until {
                params["waitUntil"] = serde_json::json!(wait_until.as_str());
            }
        } else {
            // No options provided, set default timeout (required in Playwright 1.56.1+)
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        // Send goto RPC to Frame
        // The server returns { "response": { "guid": "..." } } or null
        #[derive(Deserialize)]
        struct GotoResponse {
            response: Option<ResponseReference>,
        }

        #[derive(Deserialize)]
        struct ResponseReference {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        let goto_result: GotoResponse = self.channel().send("goto", params).await?;

        // If navigation returned a response, get the Response object from the connection
        if let Some(response_ref) = goto_result.response {
            // The server returns a Response GUID, but the __create__ message might not have
            // arrived yet. Retry a few times to wait for the object to be created.
            // TODO(Phase 4+): Implement proper GUID replacement like Python's _replace_guids_with_channels
            //   - Eliminates retry loop for better performance
            //   - See: playwright-python's _replace_guids_with_channels method
            let response_arc = {
                let mut attempts = 0;
                let max_attempts = 20; // 20 * 50ms = 1 second max wait
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

            // Extract Response data from the initializer, and store the Arc for RPC calls
            // (body(), rawHeaders(), headerValue()) that need to contact the server.
            let initializer = response_arc.initializer();

            // Extract response data from initializer
            let status = initializer["status"].as_u64().ok_or_else(|| {
                crate::error::Error::ProtocolError("Response missing status".to_string())
            })? as u16;

            // Convert headers from array format to HashMap
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

            Ok(Some(Response::new(
                initializer["url"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::Error::ProtocolError("Response missing url".to_string())
                    })?
                    .to_string(),
                status,
                initializer["statusText"].as_str().unwrap_or("").to_string(),
                headers,
                Some(response_arc),
            )))
        } else {
            // Navigation returned null (e.g., data URLs, about:blank)
            // This is a valid result, not an error
            Ok(None)
        }
    }

    /// Returns the frame's title.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-title>
    pub async fn title(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct TitleResponse {
            value: String,
        }

        let response: TitleResponse = self.channel().send("title", serde_json::json!({})).await?;
        Ok(response.value)
    }

    /// Returns the full HTML content of the frame, including the DOCTYPE.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-content>
    pub async fn content(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct ContentResponse {
            value: String,
        }

        let response: ContentResponse = self
            .channel()
            .send("content", serde_json::json!({}))
            .await?;
        Ok(response.value)
    }

    /// Sets the content of the frame.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-set-content>
    pub async fn set_content(&self, html: &str, options: Option<GotoOptions>) -> Result<()> {
        let mut params = serde_json::json!({
            "html": html,
        });

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

        self.channel().send_no_result("setContent", params).await
    }

    /// Waits for the required load state to be reached.
    ///
    /// Playwright's protocol doesn't expose `waitForLoadState` as a server-side command —
    /// it's implemented client-side using lifecycle events. We implement it by polling
    /// `document.readyState` via JavaScript evaluation.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-wait-for-load-state>
    pub async fn wait_for_load_state(&self, state: Option<WaitUntil>) -> Result<()> {
        let target_state = state.unwrap_or(WaitUntil::Load);

        let js_check = match target_state {
            // "load" means the full page has loaded (readyState === "complete")
            WaitUntil::Load => "document.readyState === 'complete'",
            // "domcontentloaded" means DOM is ready (readyState !== "loading")
            WaitUntil::DomContentLoaded => "document.readyState !== 'loading'",
            // "networkidle" has no direct readyState equivalent; we approximate
            // by checking "complete" (same as Load)
            WaitUntil::NetworkIdle => "document.readyState === 'complete'",
            // "commit" means any response has been received (readyState !== "loading" at minimum)
            WaitUntil::Commit => "document.readyState !== 'loading'",
        };

        let timeout_ms = crate::DEFAULT_TIMEOUT_MS as u64;
        let poll_interval = std::time::Duration::from_millis(50);
        let start = std::time::Instant::now();

        loop {
            #[derive(Deserialize)]
            struct EvalResponse {
                value: serde_json::Value,
            }

            let result: EvalResponse = self
                .channel()
                .send(
                    "evaluateExpression",
                    serde_json::json!({
                        "expression": js_check,
                        "isFunction": false,
                        "arg": crate::protocol::serialize_null(),
                    }),
                )
                .await?;

            // Playwright protocol returns booleans as {"b": true/false}
            let is_ready = result
                .value
                .as_object()
                .and_then(|m| m.get("b"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if is_ready {
                return Ok(());
            }

            if start.elapsed().as_millis() as u64 >= timeout_ms {
                return Err(crate::error::Error::Timeout(format!(
                    "wait_for_load_state({}) timed out after {}ms",
                    target_state.as_str(),
                    timeout_ms
                )));
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Waits for the frame to navigate to a URL matching the given string or glob pattern.
    ///
    /// Playwright's protocol doesn't expose `waitForURL` as a server-side command —
    /// it's implemented client-side. We implement it by polling `window.location.href`.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-wait-for-url>
    pub async fn wait_for_url(&self, url: &str, options: Option<GotoOptions>) -> Result<()> {
        let timeout_ms = options
            .as_ref()
            .and_then(|o| o.timeout)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(crate::DEFAULT_TIMEOUT_MS as u64);

        // Convert glob pattern to regex for matching
        // Playwright supports string (exact), glob (**), and regex patterns
        // We support exact string and basic glob patterns
        let is_glob = url.contains('*');

        let poll_interval = std::time::Duration::from_millis(50);
        let start = std::time::Instant::now();

        loop {
            let current_url = self.url();

            let matches = if is_glob {
                glob_match(url, &current_url)
            } else {
                current_url == url
            };

            if matches {
                // URL matches — optionally wait for load state
                if let Some(ref opts) = options
                    && let Some(wait_until) = opts.wait_until
                {
                    self.wait_for_load_state(Some(wait_until)).await?;
                }
                return Ok(());
            }

            if start.elapsed().as_millis() as u64 >= timeout_ms {
                return Err(crate::error::Error::Timeout(format!(
                    "wait_for_url({}) timed out after {}ms, current URL: {}",
                    url, timeout_ms, current_url
                )));
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Returns the first element matching the selector, or None if not found.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-query-selector>
    pub async fn query_selector(
        &self,
        selector: &str,
    ) -> Result<Option<Arc<crate::protocol::ElementHandle>>> {
        let response: serde_json::Value = self
            .channel()
            .send(
                "querySelector",
                serde_json::json!({
                    "selector": selector
                }),
            )
            .await?;

        // Check if response is empty (no element found)
        if response.as_object().map(|o| o.is_empty()).unwrap_or(true) {
            return Ok(None);
        }

        // Try different possible field names
        let element_value = if let Some(elem) = response.get("element") {
            elem
        } else if let Some(elem) = response.get("handle") {
            elem
        } else {
            // Maybe the response IS the guid object itself
            &response
        };

        if element_value.is_null() {
            return Ok(None);
        }

        // Element response contains { guid: "elementHandle@123" }
        let guid = element_value["guid"].as_str().ok_or_else(|| {
            crate::error::Error::ProtocolError("Element GUID missing".to_string())
        })?;

        // Look up the ElementHandle object in the connection's object registry and downcast
        let connection = self.base.connection();
        let handle: crate::protocol::ElementHandle = connection
            .get_typed::<crate::protocol::ElementHandle>(guid)
            .await?;

        Ok(Some(Arc::new(handle)))
    }

    /// Returns all elements matching the selector.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-query-selector-all>
    pub async fn query_selector_all(
        &self,
        selector: &str,
    ) -> Result<Vec<Arc<crate::protocol::ElementHandle>>> {
        #[derive(Deserialize)]
        struct QueryAllResponse {
            elements: Vec<serde_json::Value>,
        }

        let response: QueryAllResponse = self
            .channel()
            .send(
                "querySelectorAll",
                serde_json::json!({
                    "selector": selector
                }),
            )
            .await?;

        // Convert GUID responses to ElementHandle objects
        let connection = self.base.connection();
        let mut handles = Vec::new();

        for element_value in response.elements {
            let guid = element_value["guid"].as_str().ok_or_else(|| {
                crate::error::Error::ProtocolError("Element GUID missing".to_string())
            })?;

            let handle: crate::protocol::ElementHandle = connection
                .get_typed::<crate::protocol::ElementHandle>(guid)
                .await?;

            handles.push(Arc::new(handle));
        }

        Ok(handles)
    }

    // Locator delegate methods
    // These are called by Locator to perform actual queries

    /// Returns the number of elements matching the selector.
    pub(crate) async fn locator_count(&self, selector: &str) -> Result<usize> {
        // Use querySelectorAll which returns array of element handles
        #[derive(Deserialize)]
        struct QueryAllResponse {
            elements: Vec<serde_json::Value>,
        }

        let response: QueryAllResponse = self
            .channel()
            .send(
                "querySelectorAll",
                serde_json::json!({
                    "selector": selector
                }),
            )
            .await?;

        Ok(response.elements.len())
    }

    /// Returns the text content of the element.
    pub(crate) async fn locator_text_content(&self, selector: &str) -> Result<Option<String>> {
        #[derive(Deserialize)]
        struct TextContentResponse {
            value: Option<String>,
        }

        let response: TextContentResponse = self
            .channel()
            .send(
                "textContent",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns the inner text of the element.
    pub(crate) async fn locator_inner_text(&self, selector: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct InnerTextResponse {
            value: String,
        }

        let response: InnerTextResponse = self
            .channel()
            .send(
                "innerText",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns the inner HTML of the element.
    pub(crate) async fn locator_inner_html(&self, selector: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct InnerHTMLResponse {
            value: String,
        }

        let response: InnerHTMLResponse = self
            .channel()
            .send(
                "innerHTML",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns the value of the specified attribute.
    pub(crate) async fn locator_get_attribute(
        &self,
        selector: &str,
        name: &str,
    ) -> Result<Option<String>> {
        #[derive(Deserialize)]
        struct GetAttributeResponse {
            value: Option<String>,
        }

        let response: GetAttributeResponse = self
            .channel()
            .send(
                "getAttribute",
                serde_json::json!({
                    "selector": selector,
                    "name": name,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns whether the element is visible.
    pub(crate) async fn locator_is_visible(&self, selector: &str) -> Result<bool> {
        #[derive(Deserialize)]
        struct IsVisibleResponse {
            value: bool,
        }

        let response: IsVisibleResponse = self
            .channel()
            .send(
                "isVisible",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns whether the element is enabled.
    pub(crate) async fn locator_is_enabled(&self, selector: &str) -> Result<bool> {
        #[derive(Deserialize)]
        struct IsEnabledResponse {
            value: bool,
        }

        let response: IsEnabledResponse = self
            .channel()
            .send(
                "isEnabled",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns whether the checkbox or radio button is checked.
    pub(crate) async fn locator_is_checked(&self, selector: &str) -> Result<bool> {
        #[derive(Deserialize)]
        struct IsCheckedResponse {
            value: bool,
        }

        let response: IsCheckedResponse = self
            .channel()
            .send(
                "isChecked",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns whether the element is editable.
    pub(crate) async fn locator_is_editable(&self, selector: &str) -> Result<bool> {
        #[derive(Deserialize)]
        struct IsEditableResponse {
            value: bool,
        }

        let response: IsEditableResponse = self
            .channel()
            .send(
                "isEditable",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns whether the element is hidden.
    pub(crate) async fn locator_is_hidden(&self, selector: &str) -> Result<bool> {
        #[derive(Deserialize)]
        struct IsHiddenResponse {
            value: bool,
        }

        let response: IsHiddenResponse = self
            .channel()
            .send(
                "isHidden",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns whether the element is disabled.
    pub(crate) async fn locator_is_disabled(&self, selector: &str) -> Result<bool> {
        #[derive(Deserialize)]
        struct IsDisabledResponse {
            value: bool,
        }

        let response: IsDisabledResponse = self
            .channel()
            .send(
                "isDisabled",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await?;

        Ok(response.value)
    }

    /// Returns whether the element is focused (currently has focus).
    ///
    /// This implementation checks if the element is the activeElement in the DOM
    /// using JavaScript evaluation, since Playwright doesn't expose isFocused() at
    /// the protocol level.
    pub(crate) async fn locator_is_focused(&self, selector: &str) -> Result<bool> {
        #[derive(Deserialize)]
        struct EvaluateResult {
            value: serde_json::Value,
        }

        // Use JavaScript to check if the element is the active element
        // The script queries the DOM and returns true/false
        let script = r#"selector => {
                const elements = document.querySelectorAll(selector);
                if (elements.length === 0) return false;
                const element = elements[0];
                return document.activeElement === element;
            }"#;

        let params = serde_json::json!({
            "expression": script,
            "arg": {
                "value": {"s": selector},
                "handles": []
            }
        });

        let result: EvaluateResult = self.channel().send("evaluateExpression", params).await?;

        // Playwright protocol returns booleans as {"b": true} or {"b": false}
        if let serde_json::Value::Object(map) = &result.value
            && let Some(b) = map.get("b").and_then(|v| v.as_bool())
        {
            return Ok(b);
        }

        // Fallback: check if the string representation is "true"
        Ok(result.value.to_string().to_lowercase().contains("true"))
    }

    // Action delegate methods

    /// Clicks the element matching the selector.
    pub(crate) async fn locator_click(
        &self,
        selector: &str,
        options: Option<crate::protocol::ClickOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel()
            .send_no_result("click", params)
            .await
            .map_err(|e| match e {
                Error::Timeout(msg) => {
                    Error::Timeout(format!("{} (selector: '{}')", msg, selector))
                }
                other => other,
            })
    }

    /// Double clicks the element matching the selector.
    pub(crate) async fn locator_dblclick(
        &self,
        selector: &str,
        options: Option<crate::protocol::ClickOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("dblclick", params).await
    }

    /// Fills the element with text.
    pub(crate) async fn locator_fill(
        &self,
        selector: &str,
        text: &str,
        options: Option<crate::protocol::FillOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "value": text,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("fill", params).await
    }

    /// Clears the element's value.
    pub(crate) async fn locator_clear(
        &self,
        selector: &str,
        options: Option<crate::protocol::FillOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "value": "",
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("fill", params).await
    }

    /// Presses a key on the element.
    pub(crate) async fn locator_press(
        &self,
        selector: &str,
        key: &str,
        options: Option<crate::protocol::PressOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "key": key,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("press", params).await
    }

    /// Sets focus on the element matching the selector.
    pub(crate) async fn locator_focus(&self, selector: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "focus",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await
    }

    /// Removes focus from the element matching the selector.
    pub(crate) async fn locator_blur(&self, selector: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "blur",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS
                }),
            )
            .await
    }

    /// Types text into the element character by character.
    ///
    /// Uses the Playwright protocol `"type"` message (the legacy name for pressSequentially).
    pub(crate) async fn locator_press_sequentially(
        &self,
        selector: &str,
        text: &str,
        options: Option<crate::protocol::PressSequentiallyOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "text": text,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("type", params).await
    }

    /// Returns the inner text of all elements matching the selector.
    pub(crate) async fn locator_all_inner_texts(&self, selector: &str) -> Result<Vec<String>> {
        #[derive(serde::Deserialize)]
        struct EvaluateResult {
            value: serde_json::Value,
        }

        // The Playwright protocol's evalOnSelectorAll requires an `arg` field.
        // We pass a null argument since our expression doesn't use one.
        let params = serde_json::json!({
            "selector": selector,
            "expression": "ee => ee.map(e => e.innerText)",
            "isFunction": true,
            "arg": {
                "value": {"v": "null"},
                "handles": []
            }
        });

        let result: EvaluateResult = self.channel().send("evalOnSelectorAll", params).await?;

        Self::parse_string_array(result.value)
    }

    /// Returns the text content of all elements matching the selector.
    pub(crate) async fn locator_all_text_contents(&self, selector: &str) -> Result<Vec<String>> {
        #[derive(serde::Deserialize)]
        struct EvaluateResult {
            value: serde_json::Value,
        }

        // The Playwright protocol's evalOnSelectorAll requires an `arg` field.
        // We pass a null argument since our expression doesn't use one.
        let params = serde_json::json!({
            "selector": selector,
            "expression": "ee => ee.map(e => e.textContent || '')",
            "isFunction": true,
            "arg": {
                "value": {"v": "null"},
                "handles": []
            }
        });

        let result: EvaluateResult = self.channel().send("evalOnSelectorAll", params).await?;

        Self::parse_string_array(result.value)
    }

    /// Performs a touch-tap on the element matching the selector.
    ///
    /// Sends touch events rather than mouse events. Requires the browser context to be
    /// created with `has_touch: true`.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-tap>
    pub(crate) async fn locator_tap(
        &self,
        selector: &str,
        options: Option<crate::protocol::TapOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("tap", params).await
    }

    /// Drags the source element onto the target element.
    ///
    /// Both selectors must resolve to elements in this frame.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-drag-to>
    pub(crate) async fn locator_drag_to(
        &self,
        source_selector: &str,
        target_selector: &str,
        options: Option<crate::protocol::DragToOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "source": source_selector,
            "target": target_selector,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("dragAndDrop", params).await
    }

    /// Waits for the element to satisfy a state condition.
    ///
    /// Uses Playwright's `waitForSelector` RPC. The element state defaults to `visible`
    /// if not specified.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-wait-for>
    pub(crate) async fn locator_wait_for(
        &self,
        selector: &str,
        options: Option<crate::protocol::WaitForOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            // Default: wait for visible with default timeout
            params["state"] = serde_json::json!("visible");
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        // waitForSelector returns an ElementHandle or null — we discard the return value
        let _: serde_json::Value = self.channel().send("waitForSelector", params).await?;
        Ok(())
    }

    /// Evaluates a JavaScript expression in the scope of the element matching the selector.
    ///
    /// The element is passed as the first argument to the expression. This is equivalent
    /// to Playwright's `evalOnSelector` protocol call with `strict: true`.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-evaluate>
    pub(crate) async fn locator_evaluate<T: serde::Serialize>(
        &self,
        selector: &str,
        expression: &str,
        arg: Option<T>,
    ) -> Result<serde_json::Value> {
        let serialized_arg = match arg {
            Some(a) => serialize_argument(&a),
            None => serialize_null(),
        };

        let params = serde_json::json!({
            "selector": selector,
            "expression": expression,
            "isFunction": true,
            "arg": serialized_arg,
            "strict": true
        });

        #[derive(Deserialize)]
        struct EvaluateResult {
            value: serde_json::Value,
        }

        let result: EvaluateResult = self.channel().send("evalOnSelector", params).await?;
        Ok(parse_result(&result.value))
    }

    /// Evaluates a JavaScript expression in the scope of all elements matching the selector.
    ///
    /// The array of all matching elements is passed as the first argument to the expression.
    /// This is equivalent to Playwright's `evalOnSelectorAll` protocol call.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-evaluate-all>
    pub(crate) async fn locator_evaluate_all<T: serde::Serialize>(
        &self,
        selector: &str,
        expression: &str,
        arg: Option<T>,
    ) -> Result<serde_json::Value> {
        let serialized_arg = match arg {
            Some(a) => serialize_argument(&a),
            None => serialize_null(),
        };

        let params = serde_json::json!({
            "selector": selector,
            "expression": expression,
            "isFunction": true,
            "arg": serialized_arg
        });

        #[derive(Deserialize)]
        struct EvaluateResult {
            value: serde_json::Value,
        }

        let result: EvaluateResult = self.channel().send("evalOnSelectorAll", params).await?;
        Ok(parse_result(&result.value))
    }

    /// Parses a Playwright protocol array value into a Vec<String>.
    ///
    /// The Playwright protocol returns arrays as:
    /// `{"a": [{"s": "value1"}, {"s": "value2"}, ...]}`
    fn parse_string_array(value: serde_json::Value) -> Result<Vec<String>> {
        // Playwright protocol wraps arrays in {"a": [...]}
        let array = if let Some(arr) = value.get("a").and_then(|v| v.as_array()) {
            arr.clone()
        } else if let Some(arr) = value.as_array() {
            arr.clone()
        } else {
            return Ok(Vec::new());
        };

        let mut result = Vec::with_capacity(array.len());
        for item in &array {
            // Each string item is wrapped as {"s": "value"} in Playwright protocol
            let s = if let Some(s) = item.get("s").and_then(|v| v.as_str()) {
                s.to_string()
            } else if let Some(s) = item.as_str() {
                s.to_string()
            } else if item.is_null() {
                String::new()
            } else {
                item.to_string()
            };
            result.push(s);
        }
        Ok(result)
    }

    pub(crate) async fn locator_check(
        &self,
        selector: &str,
        options: Option<crate::protocol::CheckOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("check", params).await
    }

    pub(crate) async fn locator_uncheck(
        &self,
        selector: &str,
        options: Option<crate::protocol::CheckOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("uncheck", params).await
    }

    pub(crate) async fn locator_hover(
        &self,
        selector: &str,
        options: Option<crate::protocol::HoverOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "selector": selector,
            "strict": true
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("hover", params).await
    }

    pub(crate) async fn locator_input_value(&self, selector: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct InputValueResponse {
            value: String,
        }

        let response: InputValueResponse = self
            .channel()
            .send(
                "inputValue",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS  // Required in Playwright 1.56.1+
                }),
            )
            .await?;

        Ok(response.value)
    }

    pub(crate) async fn locator_select_option(
        &self,
        selector: &str,
        value: crate::protocol::SelectOption,
        options: Option<crate::protocol::SelectOptions>,
    ) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct SelectOptionResponse {
            values: Vec<String>,
        }

        let mut params = serde_json::json!({
            "selector": selector,
            "strict": true,
            "options": [value.to_json()]
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            // No options provided, add default timeout (required in Playwright 1.56.1+)
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        let response: SelectOptionResponse = self.channel().send("selectOption", params).await?;

        Ok(response.values)
    }

    pub(crate) async fn locator_select_option_multiple(
        &self,
        selector: &str,
        values: Vec<crate::protocol::SelectOption>,
        options: Option<crate::protocol::SelectOptions>,
    ) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct SelectOptionResponse {
            values: Vec<String>,
        }

        let values_array: Vec<_> = values.iter().map(|v| v.to_json()).collect();

        let mut params = serde_json::json!({
            "selector": selector,
            "strict": true,
            "options": values_array
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        } else {
            // No options provided, add default timeout (required in Playwright 1.56.1+)
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        let response: SelectOptionResponse = self.channel().send("selectOption", params).await?;

        Ok(response.values)
    }

    pub(crate) async fn locator_set_input_files(
        &self,
        selector: &str,
        file: &std::path::PathBuf,
    ) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose};
        use std::io::Read;

        // Read file contents
        let mut file_handle = std::fs::File::open(file)?;
        let mut buffer = Vec::new();
        file_handle.read_to_end(&mut buffer)?;

        // Base64 encode the file contents
        let base64_content = general_purpose::STANDARD.encode(&buffer);

        // Get file name
        let file_name = file
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| crate::error::Error::InvalidArgument("Invalid file path".to_string()))?;

        self.channel()
            .send_no_result(
                "setInputFiles",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS,  // Required in Playwright 1.56.1+
                    "payloads": [{
                        "name": file_name,
                        "buffer": base64_content
                    }]
                }),
            )
            .await
    }

    pub(crate) async fn locator_set_input_files_multiple(
        &self,
        selector: &str,
        files: &[&std::path::PathBuf],
    ) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose};
        use std::io::Read;

        // If empty array, clear the files
        if files.is_empty() {
            return self
                .channel()
                .send_no_result(
                    "setInputFiles",
                    serde_json::json!({
                        "selector": selector,
                        "strict": true,
                        "timeout": crate::DEFAULT_TIMEOUT_MS,  // Required in Playwright 1.56.1+
                        "payloads": []
                    }),
                )
                .await;
        }

        // Read and encode each file
        let mut file_objects = Vec::new();
        for file_path in files {
            let mut file_handle = std::fs::File::open(file_path)?;
            let mut buffer = Vec::new();
            file_handle.read_to_end(&mut buffer)?;

            let base64_content = general_purpose::STANDARD.encode(&buffer);
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    crate::error::Error::InvalidArgument("Invalid file path".to_string())
                })?;

            file_objects.push(serde_json::json!({
                "name": file_name,
                "buffer": base64_content
            }));
        }

        self.channel()
            .send_no_result(
                "setInputFiles",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS,  // Required in Playwright 1.56.1+
                    "payloads": file_objects
                }),
            )
            .await
    }

    pub(crate) async fn locator_set_input_files_payload(
        &self,
        selector: &str,
        file: crate::protocol::FilePayload,
    ) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose};

        // Base64 encode the file contents
        let base64_content = general_purpose::STANDARD.encode(&file.buffer);

        self.channel()
            .send_no_result(
                "setInputFiles",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS,
                    "payloads": [{
                        "name": file.name,
                        "mimeType": file.mime_type,
                        "buffer": base64_content
                    }]
                }),
            )
            .await
    }

    pub(crate) async fn locator_set_input_files_payload_multiple(
        &self,
        selector: &str,
        files: &[crate::protocol::FilePayload],
    ) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose};

        // If empty array, clear the files
        if files.is_empty() {
            return self
                .channel()
                .send_no_result(
                    "setInputFiles",
                    serde_json::json!({
                        "selector": selector,
                        "strict": true,
                        "timeout": crate::DEFAULT_TIMEOUT_MS,
                        "payloads": []
                    }),
                )
                .await;
        }

        // Encode each file
        let file_objects: Vec<_> = files
            .iter()
            .map(|file| {
                let base64_content = general_purpose::STANDARD.encode(&file.buffer);
                serde_json::json!({
                    "name": file.name,
                    "mimeType": file.mime_type,
                    "buffer": base64_content
                })
            })
            .collect();

        self.channel()
            .send_no_result(
                "setInputFiles",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "timeout": crate::DEFAULT_TIMEOUT_MS,
                    "payloads": file_objects
                }),
            )
            .await
    }

    /// Evaluates JavaScript expression in the frame context (without return value).
    ///
    /// This is used internally by Page.evaluate().
    pub(crate) async fn frame_evaluate_expression(&self, expression: &str) -> Result<()> {
        let params = serde_json::json!({
            "expression": expression,
            "arg": {
                "value": {"v": "null"},
                "handles": []
            }
        });

        let _: serde_json::Value = self.channel().send("evaluateExpression", params).await?;
        Ok(())
    }

    /// Evaluates JavaScript expression and returns the result as a String.
    ///
    /// The return value is automatically converted to a string representation.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript code to evaluate
    ///
    /// # Returns
    ///
    /// The result as a String
    pub(crate) async fn frame_evaluate_expression_value(&self, expression: &str) -> Result<String> {
        let params = serde_json::json!({
            "expression": expression,
            "arg": {
                "value": {"v": "null"},
                "handles": []
            }
        });

        #[derive(Deserialize)]
        struct EvaluateResult {
            value: serde_json::Value,
        }

        let result: EvaluateResult = self.channel().send("evaluateExpression", params).await?;

        // Playwright protocol returns values in a wrapped format:
        // - String: {"s": "value"}
        // - Number: {"n": 123}
        // - Boolean: {"b": true}
        // - Null: {"v": "null"}
        // - Undefined: {"v": "undefined"}
        match &result.value {
            Value::Object(map) => {
                if let Some(s) = map.get("s").and_then(|v| v.as_str()) {
                    // String value
                    Ok(s.to_string())
                } else if let Some(n) = map.get("n") {
                    // Number value
                    Ok(n.to_string())
                } else if let Some(b) = map.get("b").and_then(|v| v.as_bool()) {
                    // Boolean value
                    Ok(b.to_string())
                } else if let Some(v) = map.get("v").and_then(|v| v.as_str()) {
                    // null or undefined
                    Ok(v.to_string())
                } else {
                    // Unknown format, return JSON
                    Ok(result.value.to_string())
                }
            }
            _ => {
                // Fallback for unexpected formats
                Ok(result.value.to_string())
            }
        }
    }

    /// Evaluates a JavaScript expression in the frame context with optional arguments.
    ///
    /// Executes the provided JavaScript expression within the frame's context and returns
    /// the result. The return value must be JSON-serializable.
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
    /// # Example
    ///
    /// ```ignore
    /// use serde_json::json;
    /// use playwright_rs::protocol::Playwright;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let playwright = Playwright::launch().await?;
    ///     let browser = playwright.chromium().launch().await?;
    ///     let page = browser.new_page().await?;
    ///     let frame = page.main_frame().await?;
    ///
    ///     // Evaluate without arguments
    ///     let result = frame.evaluate::<()>("1 + 1", None).await?;
    ///
    ///     // Evaluate with argument
    ///     let arg = json!({"x": 5, "y": 3});
    ///     let result = frame.evaluate::<serde_json::Value>("(arg) => arg.x + arg.y", Some(&arg)).await?;
    ///     assert_eq!(result, json!(8));
    ///     Ok(())
    /// }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-evaluate>
    pub async fn evaluate<T: serde::Serialize>(
        &self,
        expression: &str,
        arg: Option<&T>,
    ) -> Result<Value> {
        // Serialize the argument
        let serialized_arg = match arg {
            Some(a) => serialize_argument(a),
            None => serialize_null(),
        };

        // Build the parameters
        let params = serde_json::json!({
            "expression": expression,
            "arg": serialized_arg
        });

        // Send the evaluateExpression command
        #[derive(Deserialize)]
        struct EvaluateResult {
            value: serde_json::Value,
        }

        let result: EvaluateResult = self.channel().send("evaluateExpression", params).await?;

        // Deserialize the result using parse_result
        Ok(parse_result(&result.value))
    }

    /// Adds a `<style>` tag into the page with the desired content.
    ///
    /// # Arguments
    ///
    /// * `options` - Style tag options (content, url, or path)
    ///
    /// At least one of `content`, `url`, or `path` must be specified.
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
    /// # let frame = page.main_frame().await?;
    /// use playwright_rs::protocol::AddStyleTagOptions;
    ///
    /// // With inline CSS
    /// frame.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .content("body { background-color: red; }")
    ///         .build()
    /// ).await?;
    ///
    /// // With URL
    /// frame.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .url("https://example.com/style.css")
    ///         .build()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-add-style-tag>
    pub async fn add_style_tag(
        &self,
        options: crate::protocol::page::AddStyleTagOptions,
    ) -> Result<Arc<crate::protocol::ElementHandle>> {
        // Validate that at least one option is provided
        options.validate()?;

        // Build protocol parameters
        let mut params = serde_json::json!({});

        if let Some(content) = &options.content {
            params["content"] = serde_json::json!(content);
        }

        if let Some(url) = &options.url {
            params["url"] = serde_json::json!(url);
        }

        if let Some(path) = &options.path {
            // Read file content and send as content
            let css_content = tokio::fs::read_to_string(path).await.map_err(|e| {
                Error::InvalidArgument(format!("Failed to read CSS file '{}': {}", path, e))
            })?;
            params["content"] = serde_json::json!(css_content);
        }

        #[derive(Deserialize)]
        struct AddStyleTagResponse {
            element: serde_json::Value,
        }

        let response: AddStyleTagResponse = self.channel().send("addStyleTag", params).await?;

        let guid = response.element["guid"].as_str().ok_or_else(|| {
            Error::ProtocolError("Element GUID missing in addStyleTag response".to_string())
        })?;

        let connection = self.base.connection();
        let handle: crate::protocol::ElementHandle = connection
            .get_typed::<crate::protocol::ElementHandle>(guid)
            .await?;

        Ok(Arc::new(handle))
    }

    /// Dispatches a DOM event on the element matching the selector.
    ///
    /// Unlike clicking or typing, `dispatch_event` directly sends the event without
    /// performing any actionability checks. It still waits for the element to be present
    /// in the DOM.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-dispatch-event>
    pub(crate) async fn locator_dispatch_event(
        &self,
        selector: &str,
        type_: &str,
        event_init: Option<serde_json::Value>,
    ) -> Result<()> {
        // Serialize eventInit using Playwright's protocol argument format.
        // If None, use {"value": {"v": "undefined"}, "handles": []}.
        let event_init_serialized = match event_init {
            Some(v) => serialize_argument(&v),
            None => serde_json::json!({"value": {"v": "undefined"}, "handles": []}),
        };

        let params = serde_json::json!({
            "selector": selector,
            "type": type_,
            "eventInit": event_init_serialized,
            "strict": true,
            "timeout": crate::DEFAULT_TIMEOUT_MS
        });

        self.channel().send_no_result("dispatchEvent", params).await
    }

    /// Returns the bounding box of the element matching the selector, or None if not visible.
    ///
    /// The bounding box is returned in pixels. If the element is not visible (e.g.,
    /// `display: none`), returns `None`.
    ///
    /// Implemented via ElementHandle because `boundingBox` is an ElementHandle-level
    /// protocol method, not a Frame-level method.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-bounding-box>
    pub(crate) async fn locator_bounding_box(
        &self,
        selector: &str,
    ) -> Result<Option<crate::protocol::locator::BoundingBox>> {
        let element = self.query_selector(selector).await?;
        match element {
            Some(handle) => handle.bounding_box().await,
            None => Ok(None),
        }
    }

    /// Scrolls the element into view if it is not already visible in the viewport.
    ///
    /// Implemented via ElementHandle because `scrollIntoViewIfNeeded` is an
    /// ElementHandle-level protocol method, not a Frame-level method.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-scroll-into-view-if-needed>
    pub(crate) async fn locator_scroll_into_view_if_needed(&self, selector: &str) -> Result<()> {
        let element = self.query_selector(selector).await?;
        match element {
            Some(handle) => handle.scroll_into_view_if_needed().await,
            None => Err(crate::error::Error::ElementNotFound(format!(
                "Element not found: {}",
                selector
            ))),
        }
    }

    /// Calls the Playwright server's `expect` method on the Frame channel.
    ///
    /// Used for assertions that are auto-retried server-side (e.g. `to.match.aria`).
    /// Returns `Ok(())` when the assertion passes, or an error containing the
    /// server-supplied `errorMessage` when the assertion fails or times out.
    pub(crate) async fn frame_expect(
        &self,
        selector: &str,
        expression: &str,
        expected_value: serde_json::Value,
        is_not: bool,
        timeout_ms: f64,
    ) -> Result<()> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ExpectResult {
            matches: bool,
            #[serde(default)]
            timed_out: Option<bool>,
            #[serde(default)]
            error_message: Option<String>,
        }

        let params = serde_json::json!({
            "selector": selector,
            "expression": expression,
            "expectedValue": expected_value,
            "isNot": is_not,
            "timeout": timeout_ms
        });

        let result: ExpectResult = self.channel().send("expect", params).await?;

        if result.matches != is_not {
            Ok(())
        } else {
            let msg = result
                .error_message
                .unwrap_or_else(|| format!("Assertion '{}' failed", expression));
            if result.timed_out == Some(true) {
                Err(crate::error::Error::AssertionTimeout(msg))
            } else {
                Err(crate::error::Error::AssertionFailed(msg))
            }
        }
    }

    /// Adds a `<script>` tag into the frame with the desired content.
    ///
    /// # Arguments
    ///
    /// * `options` - Script tag options (content, url, or path)
    ///
    /// At least one of `content`, `url`, or `path` must be specified.
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-add-script-tag>
    pub async fn add_script_tag(
        &self,
        options: crate::protocol::page::AddScriptTagOptions,
    ) -> Result<Arc<crate::protocol::ElementHandle>> {
        // Validate that at least one option is provided
        options.validate()?;

        // Build protocol parameters
        let mut params = serde_json::json!({});

        if let Some(content) = &options.content {
            params["content"] = serde_json::json!(content);
        }

        if let Some(url) = &options.url {
            params["url"] = serde_json::json!(url);
        }

        if let Some(path) = &options.path {
            // Read file content and send as content
            let js_content = tokio::fs::read_to_string(path).await.map_err(|e| {
                Error::InvalidArgument(format!("Failed to read JS file '{}': {}", path, e))
            })?;
            params["content"] = serde_json::json!(js_content);
        }

        if let Some(type_) = &options.type_ {
            params["type"] = serde_json::json!(type_);
        }

        #[derive(Deserialize)]
        struct AddScriptTagResponse {
            element: serde_json::Value,
        }

        let response: AddScriptTagResponse = self.channel().send("addScriptTag", params).await?;

        let guid = response.element["guid"].as_str().ok_or_else(|| {
            Error::ProtocolError("Element GUID missing in addScriptTag response".to_string())
        })?;

        let connection = self.base.connection();
        let handle: crate::protocol::ElementHandle = connection
            .get_typed::<crate::protocol::ElementHandle>(guid)
            .await?;

        Ok(Arc::new(handle))
    }
}

impl ChannelOwner for Frame {
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
                // Update frame's URL when navigation occurs (including hash changes)
                if let Some(url_value) = params.get("url")
                    && let Some(url_str) = url_value.as_str()
                {
                    // Update frame's URL
                    if let Ok(mut url) = self.url.write() {
                        *url = url_str.to_string();
                    }
                }
                // Forward frameNavigated event to page-level handlers
                let self_clone = self.clone();
                tokio::spawn(async move {
                    if let Some(page) = self_clone.page() {
                        page.trigger_framenavigated_event(self_clone).await;
                    }
                });
            }
            "loadstate" => {
                // Track which load states are active.
                // When "load" is added, fire page-level on_load handlers.
                if let Some(add) = params.get("add").and_then(|v| v.as_str())
                    && add == "load"
                {
                    let self_clone = self.clone();
                    tokio::spawn(async move {
                        if let Some(page) = self_clone.page() {
                            page.trigger_load_event().await;
                        }
                    });
                }
            }
            "detached" => {
                // Mark this frame as detached
                if let Ok(mut flag) = self.is_detached.write() {
                    *flag = true;
                }
            }
            _ => {
                // Other frame events not yet handled
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

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame").field("guid", &self.guid()).finish()
    }
}

/// Simple glob pattern matching for URL patterns.
///
/// Supports `*` (matches any characters except `/`) and `**` (matches any characters including `/`).
/// This matches Playwright's URL glob pattern behavior.
fn glob_match(pattern: &str, text: &str) -> bool {
    let regex_str = pattern
        .replace('.', "\\.")
        .replace("**", "\x00") // placeholder for **
        .replace('*', "[^/]*")
        .replace('\x00', ".*"); // restore ** as .*
    let regex_str = format!("^{}$", regex_str);
    regex::Regex::new(&regex_str)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}
