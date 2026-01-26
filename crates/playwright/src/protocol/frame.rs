// Frame protocol object
//
// Represents a frame within a page. Pages have a main frame, and can have child frames (iframes).
// Navigation and DOM operations happen on frames, not directly on pages.

use crate::error::{Error, Result};
use crate::protocol::page::{GotoOptions, Response};
use crate::protocol::{parse_result, serialize_argument, serialize_null};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::sync::{Arc, RwLock};

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
    /// Current URL of the frame
    /// Wrapped in RwLock to allow updates from events
    url: Arc<RwLock<String>>,
}

impl Frame {
    /// Creates a new Frame from protocol initialization
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

        Ok(Self { base, url })
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

            Ok(Some(Response {
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
            }))
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
        if let serde_json::Value::Object(map) = &result.value {
            if let Some(b) = map.get("b").and_then(|v| v.as_bool()) {
                return Ok(b);
            }
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
            }
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        self.channel().send_no_result("press", params).await
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
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
            if let Some(obj) = params.as_object_mut() {
                if let Some(opts_obj) = opts_json.as_object() {
                    obj.extend(opts_obj.clone());
                }
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
        let element = connection.get_object(guid).await?;

        let handle = element
            .as_any()
            .downcast_ref::<crate::protocol::ElementHandle>()
            .map(|e| Arc::new(e.clone()))
            .ok_or_else(|| {
                Error::ProtocolError(format!("Object {} is not an ElementHandle", guid))
            })?;

        Ok(handle)
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
                if let Some(url_value) = params.get("url") {
                    if let Some(url_str) = url_value.as_str() {
                        // Update frame's URL
                        if let Ok(mut url) = self.url.write() {
                            *url = url_str.to_string();
                        }
                    }
                }
            }
            _ => {
                // Other events will be handled in future phases
                // Events: loadstate, etc.
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
