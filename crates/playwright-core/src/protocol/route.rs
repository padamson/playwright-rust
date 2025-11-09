// Route protocol object
//
// Represents a route handler for network interception.
// Routes are created when page.route() matches a request.
//
// See: https://playwright.dev/docs/api/class-route

use crate::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::error::Result;
use crate::protocol::Request;
use serde_json::{json, Value};
use std::any::Any;
use std::sync::Arc;

/// Route represents a network route handler.
///
/// Routes allow intercepting, aborting, continuing, or fulfilling network requests.
///
/// See: <https://playwright.dev/docs/api/class-route>
#[derive(Clone)]
pub struct Route {
    base: ChannelOwnerImpl,
}

impl Route {
    /// Creates a new Route from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Route object.
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: String,
        initializer: Value,
    ) -> Result<Self> {
        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent.clone()),
            type_name,
            guid,
            initializer,
        );

        Ok(Self { base })
    }

    /// Returns the request that is being routed.
    ///
    /// See: <https://playwright.dev/docs/api/class-route#route-request>
    pub fn request(&self) -> Request {
        // The Route's parent is the Request object
        // Try to downcast the parent to Request
        if let Some(parent) = self.parent() {
            if let Some(request) = parent.as_any().downcast_ref::<Request>() {
                return request.clone();
            }
        }

        // Fallback: Create a stub Request from initializer data
        // This should rarely happen in practice
        let request_data = self
            .initializer()
            .get("request")
            .cloned()
            .unwrap_or_else(|| {
                serde_json::json!({
                    "url": "",
                    "method": "GET"
                })
            });

        let parent = self
            .parent()
            .unwrap_or_else(|| Arc::new(self.clone()) as Arc<dyn ChannelOwner>);

        let request_guid = request_data
            .get("guid")
            .and_then(|v| v.as_str())
            .unwrap_or("request-stub");

        Request::new(
            parent,
            "Request".to_string(),
            request_guid.to_string(),
            request_data,
        )
        .unwrap()
    }

    /// Aborts the route's request.
    ///
    /// # Arguments
    ///
    /// * `error_code` - Optional error code (default: "failed")
    ///
    /// Available error codes:
    /// - "aborted" - User-initiated cancellation
    /// - "accessdenied" - Permission denied
    /// - "addressunreachable" - Host unreachable
    /// - "blockedbyclient" - Client blocked request
    /// - "connectionaborted", "connectionclosed", "connectionfailed", "connectionrefused", "connectionreset"
    /// - "internetdisconnected"
    /// - "namenotresolved"
    /// - "timedout"
    /// - "failed" - Generic error (default)
    ///
    /// See: <https://playwright.dev/docs/api/class-route#route-abort>
    pub async fn abort(&self, error_code: Option<&str>) -> Result<()> {
        let params = json!({
            "errorCode": error_code.unwrap_or("failed")
        });

        self.channel()
            .send::<_, serde_json::Value>("abort", params)
            .await
            .map(|_| ())
    }

    /// Continues the route's request with optional modifications.
    ///
    /// # Arguments
    ///
    /// * `overrides` - Optional modifications to apply to the request
    ///
    /// See: <https://playwright.dev/docs/api/class-route#route-continue>
    pub async fn continue_(&self, _overrides: Option<ContinueOptions>) -> Result<()> {
        // For now, just continue without modifications
        // TODO: Support overrides in future implementation
        let params = json!({
            "isFallback": false
        });

        self.channel()
            .send::<_, serde_json::Value>("continue", params)
            .await
            .map(|_| ())
    }

    /// Fulfills the route's request with a custom response.
    ///
    /// # Arguments
    ///
    /// * `options` - Response configuration (status, headers, body, etc.)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::{Playwright, FulfillOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    ///
    /// // Mock API with JSON response
    /// page.route("**/api/data", |route| async move {
    ///     let options = FulfillOptions::builder()
    ///         .json(&serde_json::json!({"status": "ok"}))?
    ///         .build();
    ///     route.fulfill(Some(options)).await
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Known Issues
    ///
    /// TODO: Main document navigation (page.goto()) fulfillment may not work correctly.
    /// The implementation works for fetch/XHR requests but appears to have issues with
    /// replacing the main document. This needs further investigation of the Playwright
    /// protocol for main frame navigations. Workaround: Use fulfill() for API mocking
    /// (fetch/XHR), not for replacing entire page HTML during navigation.
    ///
    /// See: <https://playwright.dev/docs/api/class-route#route-fulfill>
    pub async fn fulfill(&self, options: Option<FulfillOptions>) -> Result<()> {
        let opts = options.unwrap_or_default();

        // Build the response object for the protocol
        let mut response = json!({});

        // Set status (default to 200)
        response["status"] = json!(opts.status.unwrap_or(200));

        // Set headers
        let mut headers = opts.headers.unwrap_or_default();

        // Calculate body and set content-length
        let body_bytes = opts.body.as_ref();
        if let Some(body) = body_bytes {
            let content_length = body.len().to_string();
            headers.insert("content-length".to_string(), content_length);
        }

        // Add Content-Type if specified
        let content_type = opts.content_type.clone();
        if let Some(ref ct) = content_type {
            headers.insert("content-type".to_string(), ct.clone());
        }

        // Convert headers to protocol format (array of {name, value} objects)
        let headers_array: Vec<Value> = headers
            .into_iter()
            .map(|(name, value)| {
                json!({
                    "name": name,
                    "value": value
                })
            })
            .collect();
        response["headers"] = json!(headers_array);

        // Set body if provided (base64 encoded)
        if let Some(body) = body_bytes {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(body);
            response["body"] = json!(encoded);
            response["isBase64"] = json!(true);
        }

        // Set contentType at top level if provided
        if let Some(ct) = content_type {
            response["contentType"] = json!(ct);
        }

        let params = json!({
            "response": response
        });

        self.channel()
            .send::<_, serde_json::Value>("fulfill", params)
            .await
            .map(|_| ())
    }
}

/// Options for continuing a request with modifications.
///
/// See: <https://playwright.dev/docs/api/class-route#route-continue>
#[derive(Debug, Clone, Default)]
pub struct ContinueOptions {
    // TODO: Add fields for request modifications
    // pub headers: Option<HashMap<String, String>>,
    // pub method: Option<String>,
    // pub post_data: Option<Vec<u8>>,
    // pub url: Option<String>,
}

/// Options for fulfilling a route with a custom response.
///
/// See: <https://playwright.dev/docs/api/class-route#route-fulfill>
#[derive(Debug, Clone, Default)]
pub struct FulfillOptions {
    /// HTTP status code (default: 200)
    pub status: Option<u16>,
    /// Response headers
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Response body as bytes
    pub body: Option<Vec<u8>>,
    /// Content-Type header value
    pub content_type: Option<String>,
}

impl FulfillOptions {
    /// Creates a new FulfillOptions builder
    pub fn builder() -> FulfillOptionsBuilder {
        FulfillOptionsBuilder::default()
    }
}

/// Builder for FulfillOptions
#[derive(Debug, Clone, Default)]
pub struct FulfillOptionsBuilder {
    status: Option<u16>,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<Vec<u8>>,
    content_type: Option<String>,
}

impl FulfillOptionsBuilder {
    /// Sets the HTTP status code
    pub fn status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets the response headers
    pub fn headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Sets the response body from bytes
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// Sets the response body from a string
    pub fn body_string(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into().into_bytes());
        self
    }

    /// Sets the response body from JSON (automatically sets content-type to application/json)
    pub fn json(mut self, value: &impl serde::Serialize) -> Result<Self> {
        let json_str = serde_json::to_string(value).map_err(|e| {
            crate::error::Error::ProtocolError(format!("JSON serialization failed: {}", e))
        })?;
        self.body = Some(json_str.into_bytes());
        self.content_type = Some("application/json".to_string());
        Ok(self)
    }

    /// Sets the Content-Type header
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Builds the FulfillOptions
    pub fn build(self) -> FulfillOptions {
        FulfillOptions {
            status: self.status,
            headers: self.headers,
            body: self.body,
            content_type: self.content_type,
        }
    }
}

impl ChannelOwner for Route {
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

    fn channel(&self) -> &crate::channel::Channel {
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
        // Route events will be handled in future phases
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Route")
            .field("guid", &self.guid())
            .field("request", &self.request().guid())
            .finish()
    }
}
