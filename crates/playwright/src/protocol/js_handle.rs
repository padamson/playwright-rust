// JSHandle protocol object
//
// Represents a handle to an in-browser JavaScript value.
// JSHandles are created via frame.evaluate_handle_js() and jshandle.get_property().
//
// Architecture:
// - ChannelOwner with GUID like "JSHandle@abc123"
// - Methods communicate via JSON-RPC over the channel
// - ElementHandle is conceptually a subtype but kept separate in this Rust implementation
//
// See: <https://playwright.dev/docs/api/class-jshandle>

use crate::error::Result;
use crate::protocol::evaluate_conversion::parse_result;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::ConnectionExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// JSHandle represents an in-browser JavaScript object.
///
/// JSHandles are created via [`Frame::evaluate_handle_js`](crate::protocol::Frame::evaluate_handle_js)
/// and [`JSHandle::get_property`]. Unlike `evaluate`, which serializes the return value,
/// `evaluate_handle_js` returns a live handle to the in-browser object.
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
/// let frame = page.main_frame().await?;
/// let handle = frame.evaluate_handle_js("() => ({ name: 'test' })").await?;
///
/// // Get the JSON-serializable value
/// let value = handle.json_value().await?;
/// assert_eq!(value["name"], "test");
///
/// // Get a specific property
/// let name_handle = handle.get_property("name").await?;
/// let name = name_handle.json_value().await?;
/// assert_eq!(name, "test");
///
/// // Release the handle
/// handle.dispose().await?;
///
/// browser.close().await?;
/// # Ok(())
/// # }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-jshandle>
#[derive(Clone)]
pub struct JSHandle {
    base: ChannelOwnerImpl,
}

impl JSHandle {
    /// Creates a new JSHandle from protocol initialization.
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a JSHandle object.
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

    /// Returns the JSON-serializable value of this handle.
    ///
    /// Serializes the JavaScript object to JSON. If the object has circular references
    /// or is not JSON-serializable (e.g., a DOM element), this will return an error.
    ///
    /// See: <https://playwright.dev/docs/api/class-jshandle#js-handle-json-value>
    pub async fn json_value(&self) -> Result<Value> {
        #[derive(Deserialize)]
        struct JsonValueResponse {
            value: Value,
        }

        let response: JsonValueResponse = self
            .base
            .channel()
            .send("jsonValue", serde_json::json!({}))
            .await?;

        Ok(parse_result(&response.value))
    }

    /// Returns a JSHandle for a named property of this object.
    ///
    /// # Arguments
    ///
    /// * `name` - The property name to retrieve
    ///
    /// See: <https://playwright.dev/docs/api/class-jshandle#js-handle-get-property>
    pub async fn get_property(&self, name: &str) -> Result<JSHandle> {
        #[derive(Deserialize)]
        struct HandleRef {
            guid: String,
        }
        #[derive(Deserialize)]
        struct GetPropertyResponse {
            handle: HandleRef,
        }

        let response: GetPropertyResponse = self
            .base
            .channel()
            .send(
                "getProperty",
                serde_json::json!({
                    "name": name
                }),
            )
            .await?;

        let guid = &response.handle.guid;
        let connection = self.base.connection();
        let mut attempts = 0;
        let max_attempts = 20;

        let handle = loop {
            match connection.get_typed::<JSHandle>(guid).await {
                Ok(h) => break h,
                Err(_) if attempts < max_attempts => {
                    attempts += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        };

        Ok(handle)
    }

    /// Returns a map of all enumerable own properties of this object.
    ///
    /// Each value in the map is a [`JSHandle`] for the property value.
    ///
    /// See: <https://playwright.dev/docs/api/class-jshandle#js-handle-get-properties>
    pub async fn get_properties(&self) -> Result<HashMap<String, JSHandle>> {
        #[derive(Deserialize)]
        struct PropertyEntry {
            name: String,
            value: HandleRef,
        }
        #[derive(Deserialize)]
        struct HandleRef {
            guid: String,
        }
        #[derive(Deserialize)]
        struct GetPropertiesResponse {
            properties: Vec<PropertyEntry>,
        }

        let response: GetPropertiesResponse = self
            .base
            .channel()
            .send("getPropertyList", serde_json::json!({}))
            .await?;

        let connection = self.base.connection();
        let mut map = HashMap::new();

        for entry in response.properties {
            let guid = &entry.name.clone();
            let handle_guid = &entry.value.guid;

            let mut attempts = 0;
            let max_attempts = 20;

            let handle = loop {
                match connection.get_typed::<JSHandle>(handle_guid).await {
                    Ok(h) => break h,
                    Err(_) if attempts < max_attempts => {
                        attempts += 1;
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                    Err(e) => return Err(e),
                }
            };

            map.insert(guid.clone(), handle);
        }

        Ok(map)
    }

    /// Evaluates a JavaScript expression with this handle as the first argument.
    ///
    /// Returns the deserialized result of the expression.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript expression (function or expression string)
    /// * `arg` - Optional additional argument to pass after the handle
    ///
    /// # Errors
    ///
    /// Returns an error if the JavaScript expression throws.
    ///
    /// See: <https://playwright.dev/docs/api/class-jshandle#js-handle-evaluate>
    pub async fn evaluate<R, T>(&self, expression: &str, arg: Option<&T>) -> Result<R>
    where
        R: DeserializeOwned,
        T: Serialize,
    {
        // The handle is passed as the first argument using the handles array protocol.
        // The argument value uses {"h": 0} to reference the first handle.
        // Any additional arg is not supported in this signature (matches playwright-python).
        let _ = arg; // arg parameter reserved for future use / compatibility

        let params = serde_json::json!({
            "expression": expression,
            "isFunction": true,
            "arg": {
                "value": {"h": 0},
                "handles": [{"guid": self.base.guid()}]
            }
        });

        #[derive(Deserialize)]
        struct EvaluateResult {
            value: Value,
        }

        let result: EvaluateResult = self
            .base
            .channel()
            .send("evaluateExpression", params)
            .await?;

        let parsed = parse_result(&result.value);
        serde_json::from_value(parsed).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to deserialize result: {}", e))
        })
    }

    /// Evaluates a JavaScript expression returning a new JSHandle.
    ///
    /// Unlike [`evaluate`](JSHandle::evaluate) which deserializes the result,
    /// this returns a handle to the in-browser object.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript expression (function or expression string)
    /// * `arg` - Optional additional argument to pass after the handle
    ///
    /// See: <https://playwright.dev/docs/api/class-jshandle#js-handle-evaluate-handle>
    pub async fn evaluate_handle<T>(&self, expression: &str, arg: Option<&T>) -> Result<JSHandle>
    where
        T: Serialize,
    {
        let _ = arg; // arg parameter reserved for future use / compatibility

        let params = serde_json::json!({
            "expression": expression,
            "isFunction": true,
            "arg": {
                "value": {"h": 0},
                "handles": [{"guid": self.base.guid()}]
            }
        });

        #[derive(Deserialize)]
        struct HandleRef {
            guid: String,
        }
        #[derive(Deserialize)]
        struct EvaluateHandleResponse {
            handle: HandleRef,
        }

        let response: EvaluateHandleResponse = self
            .base
            .channel()
            .send("evaluateExpressionHandle", params)
            .await?;

        let guid = &response.handle.guid;
        let connection = self.base.connection();
        let mut attempts = 0;
        let max_attempts = 20;

        let handle = loop {
            match connection.get_typed::<JSHandle>(guid).await {
                Ok(h) => break h,
                Err(_) if attempts < max_attempts => {
                    attempts += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        };

        Ok(handle)
    }

    /// Releases this handle and frees the associated browser resources.
    ///
    /// After calling `dispose()`, the handle is no longer valid.
    ///
    /// See: <https://playwright.dev/docs/api/class-jshandle#js-handle-dispose>
    pub async fn dispose(&self) -> Result<()> {
        self.base
            .channel()
            .send_no_result("dispose", serde_json::json!({}))
            .await
    }

    /// Returns this handle as an [`ElementHandle`](crate::protocol::ElementHandle) if it
    /// represents a DOM element, or `None` if it is a non-element JS value.
    ///
    /// This method checks whether the protocol type of this handle is `"ElementHandle"`.
    /// In Playwright's protocol, DOM element handles are typed as `ElementHandle` while
    /// plain JavaScript values are typed as `JSHandle`.
    ///
    /// See: <https://playwright.dev/docs/api/class-jshandle#js-handle-as-element>
    pub fn as_element_type_name(&self) -> Option<&str> {
        // In practice, the server will have created an ElementHandle object
        // (not a JSHandle) for DOM elements. JSHandle.as_element() returns None
        // for non-DOM values. We check the stored type_name for forward compatibility.
        if self.base.type_name() == "ElementHandle" {
            Some(self.base.guid())
        } else {
            None
        }
    }
}

impl ChannelOwner for JSHandle {
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
        // JSHandle has no events
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for JSHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSHandle")
            .field("guid", &self.guid())
            .finish()
    }
}
