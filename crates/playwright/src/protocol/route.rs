// Route protocol object
//
// Represents a route handler for network interception.
// Routes are created when page.route() or context.route() matches a request.
//
// See: https://playwright.dev/docs/api/class-route

use crate::error::Result;
use crate::protocol::Request;
use crate::protocol::api_request_context::{APIRequestContext, InnerFetchOptions};
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde_json::{Value, json};
use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Route represents a network route handler.
///
/// Routes allow intercepting, aborting, continuing, or fulfilling network requests.
///
/// See: <https://playwright.dev/docs/api/class-route>
#[derive(Clone)]
pub struct Route {
    base: ChannelOwnerImpl,
    /// Tracks whether the route has been fully handled (abort/continue/fulfill).
    /// Used by fallback() to signal that handler chaining should continue.
    handled: Arc<AtomicBool>,
    /// APIRequestContext for performing fetch operations.
    /// Set by the route event dispatcher (Page or BrowserContext).
    api_request_context: Arc<Mutex<Option<APIRequestContext>>>,
}

impl Route {
    /// Creates a new Route from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Route object.
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent.clone()),
            type_name,
            guid,
            initializer,
        );

        Ok(Self {
            base,
            handled: Arc::new(AtomicBool::new(false)),
            api_request_context: Arc::new(Mutex::new(None)),
        })
    }

    /// Returns whether this route was fully handled by a handler.
    ///
    /// Returns `false` if the handler called `fallback()`, indicating the next
    /// matching handler should be tried.
    pub(crate) fn was_handled(&self) -> bool {
        self.handled.load(Ordering::SeqCst)
    }

    /// Sets the APIRequestContext for this route, enabling `fetch()`.
    ///
    /// Called by the route event dispatcher (Page or BrowserContext) when
    /// dispatching the route to a handler.
    pub(crate) fn set_api_request_context(&self, ctx: APIRequestContext) {
        *self.api_request_context.lock().unwrap() = Some(ctx);
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
            Arc::from(request_guid),
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
        self.handled.store(true, Ordering::SeqCst);
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
    /// This is a final action — no other route handlers will run for this request.
    /// Use `fallback()` instead if you want subsequent handlers to have a chance.
    ///
    /// # Arguments
    ///
    /// * `overrides` - Optional modifications to apply to the request
    ///
    /// See: <https://playwright.dev/docs/api/class-route#route-continue>
    pub async fn continue_(&self, overrides: Option<ContinueOptions>) -> Result<()> {
        self.handled.store(true, Ordering::SeqCst);
        self.continue_internal(overrides, false).await
    }

    /// Continues the route's request, allowing subsequent handlers to run.
    ///
    /// Unlike `continue_()`, `fallback()` yields to the next matching handler in the
    /// chain before the request reaches the network. This enables middleware-like
    /// handler composition where multiple handlers can inspect and modify a request.
    ///
    /// # Arguments
    ///
    /// * `overrides` - Optional modifications to apply to the request
    ///
    /// See: <https://playwright.dev/docs/api/class-route#route-fallback>
    pub async fn fallback(&self, overrides: Option<ContinueOptions>) -> Result<()> {
        // Don't set handled — signals to the dispatcher to try the next handler
        self.continue_internal(overrides, true).await
    }

    /// Internal implementation shared by continue_() and fallback()
    async fn continue_internal(
        &self,
        overrides: Option<ContinueOptions>,
        is_fallback: bool,
    ) -> Result<()> {
        let mut params = json!({
            "isFallback": is_fallback
        });

        // Add overrides if provided
        if let Some(opts) = overrides {
            // Add headers
            if let Some(headers) = opts.headers {
                let headers_array: Vec<serde_json::Value> = headers
                    .into_iter()
                    .map(|(name, value)| json!({"name": name, "value": value}))
                    .collect();
                params["headers"] = json!(headers_array);
            }

            // Add method
            if let Some(method) = opts.method {
                params["method"] = json!(method);
            }

            // Add postData (string or binary)
            if let Some(post_data) = opts.post_data {
                params["postData"] = json!(post_data);
            } else if let Some(post_data_bytes) = opts.post_data_bytes {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&post_data_bytes);
                params["postData"] = json!(encoded);
            }

            // Add URL
            if let Some(url) = opts.url {
                params["url"] = json!(url);
            }
        }

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
    /// # Known Limitations
    ///
    /// **Response body fulfillment is not supported in Playwright 1.49.0 - 1.58.2.**
    ///
    /// The route.fulfill() method can successfully send requests for status codes and headers,
    /// but the response body is not transmitted to the browser JavaScript layer. This applies
    /// to ALL request types (main document, fetch, XHR, etc.), not just document navigation.
    ///
    /// **Investigation Findings:**
    /// - The protocol message is correctly formatted and accepted by the Playwright server
    /// - The body bytes are present in the fulfill() call
    /// - The Playwright server creates a Response object
    /// - But the body content does not reach the browser's fetch/network API
    ///
    /// This appears to be a limitation or bug in the Playwright server implementation.
    /// Tested with versions 1.49.0, 1.56.1, and 1.58.2 (latest as of 2026-03-22).
    ///
    /// TODO: Periodically test with newer Playwright versions for fix.
    /// Workaround: Mock responses at the HTTP server level rather than using network interception,
    /// or wait for a newer Playwright version that supports response body fulfillment.
    ///
    /// See: <https://playwright.dev/docs/api/class-route#route-fulfill>
    pub async fn fulfill(&self, options: Option<FulfillOptions>) -> Result<()> {
        self.handled.store(true, Ordering::SeqCst);
        let opts = options.unwrap_or_default();

        // Build the response object for the protocol
        let mut response = json!({
            "status": opts.status.unwrap_or(200),
            "headers": []
        });

        // Set headers - prepare them BEFORE adding body
        let mut headers_map = opts.headers.unwrap_or_default();

        // Set body if provided, and prepare headers
        let body_bytes = opts.body.as_ref();
        if let Some(body) = body_bytes {
            let content_length = body.len().to_string();
            headers_map.insert("content-length".to_string(), content_length);
        }

        // Add Content-Type if specified
        if let Some(ref ct) = opts.content_type {
            headers_map.insert("content-type".to_string(), ct.clone());
        }

        // Convert headers to protocol format
        let headers_array: Vec<Value> = headers_map
            .into_iter()
            .map(|(name, value)| json!({"name": name, "value": value}))
            .collect();
        response["headers"] = json!(headers_array);

        // Set body LAST, after all other fields
        if let Some(body) = body_bytes {
            // Send as plain string for text (UTF-8), base64 for binary
            if let Ok(body_str) = std::str::from_utf8(body) {
                response["body"] = json!(body_str);
            } else {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(body);
                response["body"] = json!(encoded);
                response["isBase64"] = json!(true);
            }
        }

        let params = json!({
            "response": response
        });

        self.channel()
            .send::<_, serde_json::Value>("fulfill", params)
            .await
            .map(|_| ())
    }

    /// Performs the request and fetches result without fulfilling it, so that the
    /// response can be modified and then fulfilled.
    ///
    /// Delegates to `APIRequestContext.inner_fetch()` using the request's URL and
    /// any provided overrides.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional overrides for the fetch request
    ///
    /// See: <https://playwright.dev/docs/api/class-route#route-fetch>
    pub async fn fetch(&self, options: Option<FetchOptions>) -> Result<FetchResponse> {
        self.handled.store(true, Ordering::SeqCst);

        let api_ctx = self
            .api_request_context
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "No APIRequestContext available for route.fetch(). \
                     This can happen if the route was not dispatched through \
                     a BrowserContext with an associated request context."
                        .to_string(),
                )
            })?;

        let request = self.request();
        let opts = options.unwrap_or_default();

        // Use the original request URL unless overridden
        let url = opts.url.unwrap_or_else(|| request.url().to_string());

        let inner_opts = InnerFetchOptions {
            method: opts.method.or_else(|| Some(request.method().to_string())),
            headers: opts.headers,
            post_data: opts.post_data,
            post_data_bytes: opts.post_data_bytes,
            max_redirects: opts.max_redirects,
            max_retries: opts.max_retries,
            timeout: opts.timeout,
        };

        api_ctx.inner_fetch(&url, Some(inner_opts)).await
    }
}

/// Checks if a URL matches a glob pattern.
///
/// Supports standard glob patterns:
/// - `*` matches any characters except `/`
/// - `**` matches any characters including `/`
/// - `?` matches a single character
pub(crate) fn matches_pattern(pattern: &str, url: &str) -> bool {
    use glob::Pattern;

    match Pattern::new(pattern) {
        Ok(glob_pattern) => glob_pattern.matches(url),
        Err(_) => {
            // If pattern is invalid, fall back to exact string match
            pattern == url
        }
    }
}

/// Behavior when removing route handlers via `unroute_all()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-unroute-all>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnrouteBehavior {
    /// Wait for in-flight handlers to complete before removing
    Wait,
    /// Stop handlers and ignore any errors they throw
    IgnoreErrors,
    /// Default behavior (does not wait, does not ignore errors)
    Default,
}

/// Response from `route.fetch()`, allowing inspection and modification before fulfillment.
///
/// See: <https://playwright.dev/docs/api/class-apiresponse>
#[derive(Debug, Clone)]
pub struct FetchResponse {
    /// HTTP status code
    pub status: u16,
    /// HTTP status text
    pub status_text: String,
    /// Response headers as name-value pairs
    pub headers: Vec<(String, String)>,
    /// Response body as bytes
    pub body: Vec<u8>,
}

impl FetchResponse {
    /// Returns the HTTP status code
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Returns the status text
    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    /// Returns response headers
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Returns the response body as bytes
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Returns the response body as text
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.body.clone()).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Response body is not valid UTF-8: {}", e))
        })
    }

    /// Returns the response body parsed as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.body).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to parse response JSON: {}", e))
        })
    }

    /// Returns true if status is in 200-299 range
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Options for continuing a request with modifications.
///
/// Allows modifying headers, method, post data, and URL when continuing a route.
/// Used by both `continue_()` and `fallback()`.
///
/// See: <https://playwright.dev/docs/api/class-route#route-continue>
#[derive(Debug, Clone, Default)]
pub struct ContinueOptions {
    /// Modified request headers
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Modified request method (GET, POST, etc.)
    pub method: Option<String>,
    /// Modified POST data as string
    pub post_data: Option<String>,
    /// Modified POST data as bytes
    pub post_data_bytes: Option<Vec<u8>>,
    /// Modified request URL (must have same protocol)
    pub url: Option<String>,
}

impl ContinueOptions {
    /// Creates a new builder for ContinueOptions
    pub fn builder() -> ContinueOptionsBuilder {
        ContinueOptionsBuilder::default()
    }
}

/// Builder for ContinueOptions
#[derive(Debug, Clone, Default)]
pub struct ContinueOptionsBuilder {
    headers: Option<std::collections::HashMap<String, String>>,
    method: Option<String>,
    post_data: Option<String>,
    post_data_bytes: Option<Vec<u8>>,
    url: Option<String>,
}

impl ContinueOptionsBuilder {
    /// Sets the request headers
    pub fn headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Sets the request method
    pub fn method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }

    /// Sets the POST data as a string
    pub fn post_data(mut self, post_data: String) -> Self {
        self.post_data = Some(post_data);
        self.post_data_bytes = None; // Clear bytes if setting string
        self
    }

    /// Sets the POST data as bytes
    pub fn post_data_bytes(mut self, post_data_bytes: Vec<u8>) -> Self {
        self.post_data_bytes = Some(post_data_bytes);
        self.post_data = None; // Clear string if setting bytes
        self
    }

    /// Sets the request URL (must have same protocol as original)
    pub fn url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    /// Builds the ContinueOptions
    pub fn build(self) -> ContinueOptions {
        ContinueOptions {
            headers: self.headers,
            method: self.method,
            post_data: self.post_data,
            post_data_bytes: self.post_data_bytes,
            url: self.url,
        }
    }
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

/// Options for fetching a route's request.
///
/// See: <https://playwright.dev/docs/api/class-route#route-fetch>
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// Modified request headers
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Modified request method (GET, POST, etc.)
    pub method: Option<String>,
    /// Modified POST data as string
    pub post_data: Option<String>,
    /// Modified POST data as bytes
    pub post_data_bytes: Option<Vec<u8>>,
    /// Modified request URL
    pub url: Option<String>,
    /// Maximum number of redirects to follow (default: 20)
    pub max_redirects: Option<u32>,
    /// Maximum number of retries (default: 0)
    pub max_retries: Option<u32>,
    /// Request timeout in milliseconds
    pub timeout: Option<f64>,
}

impl FetchOptions {
    /// Creates a new FetchOptions builder
    pub fn builder() -> FetchOptionsBuilder {
        FetchOptionsBuilder::default()
    }
}

/// Builder for FetchOptions
#[derive(Debug, Clone, Default)]
pub struct FetchOptionsBuilder {
    headers: Option<std::collections::HashMap<String, String>>,
    method: Option<String>,
    post_data: Option<String>,
    post_data_bytes: Option<Vec<u8>>,
    url: Option<String>,
    max_redirects: Option<u32>,
    max_retries: Option<u32>,
    timeout: Option<f64>,
}

impl FetchOptionsBuilder {
    /// Sets the request headers
    pub fn headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Sets the request method
    pub fn method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }

    /// Sets the POST data as a string
    pub fn post_data(mut self, post_data: String) -> Self {
        self.post_data = Some(post_data);
        self.post_data_bytes = None;
        self
    }

    /// Sets the POST data as bytes
    pub fn post_data_bytes(mut self, post_data_bytes: Vec<u8>) -> Self {
        self.post_data_bytes = Some(post_data_bytes);
        self.post_data = None;
        self
    }

    /// Sets the request URL
    pub fn url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    /// Sets the maximum number of redirects to follow
    pub fn max_redirects(mut self, n: u32) -> Self {
        self.max_redirects = Some(n);
        self
    }

    /// Sets the maximum number of retries
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = Some(n);
        self
    }

    /// Sets the request timeout in milliseconds
    pub fn timeout(mut self, ms: f64) -> Self {
        self.timeout = Some(ms);
        self
    }

    /// Builds the FetchOptions
    pub fn build(self) -> FetchOptions {
        FetchOptions {
            headers: self.headers,
            method: self.method,
            post_data: self.post_data,
            post_data_bytes: self.post_data_bytes,
            url: self.url,
            max_redirects: self.max_redirects,
            max_retries: self.max_retries,
            timeout: self.timeout,
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
