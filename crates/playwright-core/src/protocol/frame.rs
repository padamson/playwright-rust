// Frame protocol object
//
// Represents a frame within a page. Pages have a main frame, and can have child frames (iframes).
// Navigation and DOM operations happen on frames, not directly on pages.

use crate::channel::Channel;
use crate::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::error::Result;
use crate::protocol::page::{GotoOptions, Response};
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

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
}

impl Frame {
    /// Creates a new Frame from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Frame object.
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
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Navigates the frame to the specified URL.
    ///
    /// This is the actual protocol method for navigation. Page.goto() delegates to this.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to
    /// * `options` - Optional navigation options (timeout, wait_until)
    ///
    /// See: <https://playwright.dev/docs/api/class-frame#frame-goto>
    pub async fn goto(&self, url: &str, options: Option<GotoOptions>) -> Result<Response> {
        // Build params manually using json! macro
        let mut params = serde_json::json!({
            "url": url,
        });

        // Add optional parameters
        if let Some(opts) = options {
            if let Some(timeout) = opts.timeout {
                params["timeout"] = serde_json::json!(timeout.as_millis() as u64);
            }
            if let Some(wait_until) = opts.wait_until {
                params["waitUntil"] = serde_json::json!(wait_until.as_str());
            }
        }

        // Send goto RPC to Frame
        // The server returns { "response": { "guid": "..." } } or null
        #[derive(Deserialize)]
        struct GotoResponse {
            response: Option<ResponseReference>,
        }

        #[derive(Deserialize)]
        struct ResponseReference {
            guid: String,
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

            // Note: ResponseObject protocol object exists (crates/playwright-core/src/protocol/response.rs)
            // We extract Response data from its initializer rather than wrapping the protocol object
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

            Ok(Response {
                url: initializer["url"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::Error::ProtocolError("Response missing url".to_string())
                    })?
                    .to_string(),
                status,
                status_text: initializer["statusText"].as_str().unwrap_or("").to_string(),
                ok: (200..300).contains(&status), // Compute ok from status code
                headers,
            })
        } else {
            // Navigation returned null (e.g., about:blank or failed navigation)
            Err(crate::error::Error::ProtocolError(
                "Navigation did not return a response".to_string(),
            ))
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

    /// Returns the first element matching the selector, or None if not found.
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
    /// page.goto("https://example.com", None).await?;
    ///
    /// if let Some(element) = page.query_selector("h1").await? {
    ///     let screenshot = element.screenshot(None).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
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

        // Look up the ElementHandle object in the connection's object registry
        let connection = self.base.connection();
        let element = connection.get_object(guid).await?;

        // Downcast to ElementHandle
        let handle = element
            .as_any()
            .downcast_ref::<crate::protocol::ElementHandle>()
            .map(|e| Arc::new(e.clone()))
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(format!(
                    "Object {} is not an ElementHandle",
                    guid
                ))
            })?;

        Ok(Some(handle))
    }

    /// Returns all elements matching the selector.
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
    /// page.goto("https://example.com", None).await?;
    ///
    /// let paragraphs = page.query_selector_all("p").await?;
    /// println!("Found {} paragraphs", paragraphs.len());
    /// # Ok(())
    /// # }
    /// ```
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

            let element = connection.get_object(guid).await?;

            let handle = element
                .as_any()
                .downcast_ref::<crate::protocol::ElementHandle>()
                .map(|e| Arc::new(e.clone()))
                .ok_or_else(|| {
                    crate::error::Error::ProtocolError(format!(
                        "Object {} is not an ElementHandle",
                        guid
                    ))
                })?;

            handles.push(handle);
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
                    "strict": true
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
                    "strict": true
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
                    "strict": true
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
                    "strict": true
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
                    "strict": true
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
                    "strict": true
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
                    "strict": true
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
                    "strict": true
                }),
            )
            .await?;

        Ok(response.value)
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        }

        self.channel().send_no_result("click", params).await
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        }

        self.channel().send_no_result("dblclick", params).await
    }

    /// Fills the element with text.
    pub(crate) async fn locator_fill(&self, selector: &str, text: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "fill",
                serde_json::json!({
                    "selector": selector,
                    "value": text,
                    "strict": true
                }),
            )
            .await
    }

    /// Clears the element's value.
    pub(crate) async fn locator_clear(&self, selector: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "fill",
                serde_json::json!({
                    "selector": selector,
                    "value": "",
                    "strict": true
                }),
            )
            .await
    }

    /// Presses a key on the element.
    pub(crate) async fn locator_press(&self, selector: &str, key: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "press",
                serde_json::json!({
                    "selector": selector,
                    "key": key,
                    "strict": true
                }),
            )
            .await
    }

    pub(crate) async fn locator_check(&self, selector: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "check",
                serde_json::json!({
                    "selector": selector,
                    "strict": true
                }),
            )
            .await
    }

    pub(crate) async fn locator_uncheck(&self, selector: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "uncheck",
                serde_json::json!({
                    "selector": selector,
                    "strict": true
                }),
            )
            .await
    }

    pub(crate) async fn locator_hover(&self, selector: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "hover",
                serde_json::json!({
                    "selector": selector,
                    "strict": true
                }),
            )
            .await
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
                    "strict": true
                }),
            )
            .await?;

        Ok(response.value)
    }

    pub(crate) async fn locator_select_option(
        &self,
        selector: &str,
        value: &str,
    ) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct SelectOptionResponse {
            values: Vec<String>,
        }

        let response: SelectOptionResponse = self
            .channel()
            .send(
                "selectOption",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "options": [{"value": value}]
                }),
            )
            .await?;

        Ok(response.values)
    }

    pub(crate) async fn locator_select_option_multiple(
        &self,
        selector: &str,
        values: &[&str],
    ) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct SelectOptionResponse {
            values: Vec<String>,
        }

        let values_array: Vec<_> = values
            .iter()
            .map(|v| serde_json::json!({"value": v}))
            .collect();

        let response: SelectOptionResponse = self
            .channel()
            .send(
                "selectOption",
                serde_json::json!({
                    "selector": selector,
                    "strict": true,
                    "options": values_array
                }),
            )
            .await?;

        Ok(response.values)
    }

    pub(crate) async fn locator_set_input_files(
        &self,
        selector: &str,
        file: &std::path::PathBuf,
    ) -> Result<()> {
        use base64::{engine::general_purpose, Engine as _};
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
        use base64::{engine::general_purpose, Engine as _};
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
                    "payloads": file_objects
                }),
            )
            .await
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
        // TODO: Handle frame events in future phases
        // Events: loadstate, navigated, etc.
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
