// Request protocol object
//
// Represents an HTTP request. Created during navigation operations.
// In Playwright's architecture, navigation creates a Request which receives a Response.

use crate::error::Result;
use crate::protocol::response::HeaderEntry;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::ConnectionExt;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Request represents an HTTP request during navigation.
///
/// Request objects are created by the server during navigation operations.
/// They are parents to Response objects.
///
/// See: <https://playwright.dev/docs/api/class-request>
#[derive(Clone)]
pub struct Request {
    base: ChannelOwnerImpl,
    /// Failure text set when a `requestFailed` event is received for this request.
    failure_text: Arc<Mutex<Option<String>>>,
    /// Timing data set when the associated `requestFinished` event fires.
    /// The value is the raw JSON timing object from the Response initializer.
    timing: Arc<Mutex<Option<Value>>>,
    /// Eagerly resolved Frame back-reference from the initializer's `frame.guid`.
    frame: Arc<Mutex<Option<crate::protocol::Frame>>>,
    /// The request that redirected to this one (from initializer `redirectedFrom`).
    redirected_from: Arc<Mutex<Option<Request>>>,
    /// The request that this one redirected to (set by the later request's construction).
    redirected_to: Arc<Mutex<Option<Request>>>,
}

impl Request {
    /// Creates a new Request from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Request object.
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

        Ok(Self {
            base,
            failure_text: Arc::new(Mutex::new(None)),
            timing: Arc::new(Mutex::new(None)),
            frame: Arc::new(Mutex::new(None)),
            redirected_from: Arc::new(Mutex::new(None)),
            redirected_to: Arc::new(Mutex::new(None)),
        })
    }

    /// Returns the [`Frame`](crate::protocol::Frame) that initiated this request.
    ///
    /// The frame is resolved from the `frame` GUID in the protocol initializer data.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-frame>
    pub fn frame(&self) -> Option<crate::protocol::Frame> {
        self.frame.lock().unwrap().clone()
    }

    /// Returns the request that redirected to this one, or `None`.
    ///
    /// When the server responds with a redirect, Playwright creates a new Request
    /// for the redirect target. The new request's `redirected_from` points back to
    /// the original request.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-redirected-from>
    pub fn redirected_from(&self) -> Option<Request> {
        self.redirected_from.lock().unwrap().clone()
    }

    /// Returns the request that this one redirected to, or `None`.
    ///
    /// This is the inverse of `redirected_from()`: if request A redirected to
    /// request B, then `A.redirected_to()` returns B.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-redirected-to>
    pub fn redirected_to(&self) -> Option<Request> {
        self.redirected_to.lock().unwrap().clone()
    }

    /// Sets the redirect-from back-pointer. Called by the object factory
    /// when a new Request has `redirectedFrom` in its initializer.
    pub(crate) fn set_redirected_from(&self, from: Request) {
        *self.redirected_from.lock().unwrap() = Some(from);
    }

    /// Sets the redirect-to forward pointer. Called as a side-effect when
    /// the redirect target request is constructed.
    pub(crate) fn set_redirected_to(&self, to: Request) {
        *self.redirected_to.lock().unwrap() = Some(to);
    }

    /// Returns the [`Response`](crate::protocol::response::ResponseObject) for this request.
    ///
    /// Sends a `"response"` RPC call to the Playwright server.
    /// Returns `None` if the request has not received a response (e.g., it failed).
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-response>
    pub async fn response(&self) -> Result<Option<crate::protocol::page::Response>> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct GuidRef {
            guid: String,
        }

        #[derive(Deserialize)]
        struct ResponseResult {
            response: Option<GuidRef>,
        }

        let result: ResponseResult = self
            .channel()
            .send("response", serde_json::json!({}))
            .await?;

        let guid = match result.response {
            Some(r) => r.guid,
            None => return Ok(None),
        };

        let connection = self.connection();
        // get_typed validates the type; get_object provides the Arc<dyn ChannelOwner>
        // needed by Response::new for back-reference support
        let response_obj: crate::protocol::ResponseObject = connection
            .get_typed::<crate::protocol::ResponseObject>(&guid)
            .await
            .map_err(|e| {
                crate::error::Error::ProtocolError(format!(
                    "Failed to get Response object {}: {}",
                    guid, e
                ))
            })?;
        let response_arc = connection.get_object(&guid).await.map_err(|e| {
            crate::error::Error::ProtocolError(format!(
                "Failed to get Response object {}: {}",
                guid, e
            ))
        })?;

        let initializer = response_obj.initializer();
        let status = initializer
            .get("status")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u16;
        let headers: std::collections::HashMap<String, String> = initializer
            .get("headers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|h| {
                        let name = h.get("name")?.as_str()?;
                        let value = h.get("value")?.as_str()?;
                        Some((name.to_string(), value.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(Some(crate::protocol::page::Response::new(
            initializer
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            status,
            initializer
                .get("statusText")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            headers,
            Some(response_arc),
        )))
    }

    /// Returns resource size information for this request.
    ///
    /// Internally fetches the associated Response (via RPC) and calls `sizes()`
    /// on the response's channel.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-sizes>
    pub async fn sizes(&self) -> Result<crate::protocol::response::RequestSizes> {
        let response = self.response().await?;
        let response = response.ok_or_else(|| {
            crate::error::Error::ProtocolError(
                "Unable to fetch sizes for failed request".to_string(),
            )
        })?;

        let response_obj = response.response_object().map_err(|_| {
            crate::error::Error::ProtocolError(
                "Response has no backing protocol object for sizes()".to_string(),
            )
        })?;

        response_obj.sizes().await
    }

    /// Sets the eagerly-resolved Frame back-reference.
    ///
    /// Called by the object factory after the Request is created and the Frame
    /// has been looked up from the connection registry.
    pub(crate) fn set_frame(&self, frame: crate::protocol::Frame) {
        *self.frame.lock().unwrap() = Some(frame);
    }

    /// Returns the URL of the request.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-url>
    pub fn url(&self) -> &str {
        self.initializer()
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Returns the HTTP method of the request (GET, POST, etc.).
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-method>
    pub fn method(&self) -> &str {
        self.initializer()
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
    }

    /// Returns the resource type of the request (e.g., "document", "stylesheet", "image", "fetch", etc.).
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-resource-type>
    pub fn resource_type(&self) -> &str {
        self.initializer()
            .get("resourceType")
            .and_then(|v| v.as_str())
            .unwrap_or("other")
    }

    /// Check if this request is for a navigation (main document).
    ///
    /// A navigation request is when the request is for the main frame's document.
    /// This is used to distinguish between main document loads and subresource loads.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-is-navigation-request>
    pub fn is_navigation_request(&self) -> bool {
        self.resource_type() == "document"
    }

    /// Returns the request headers as a HashMap.
    ///
    /// The headers are read from the protocol initializer data. The format in the
    /// protocol is a list of `{name, value}` objects which are merged into a
    /// `HashMap<String, String>`. If duplicate header names exist, the last
    /// value wins.
    ///
    /// For the full set of raw headers (including duplicates), use
    /// [`headers_array()`](Self::headers_array) or [`all_headers()`](Self::all_headers).
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-headers>
    pub fn headers(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(headers) = self.initializer().get("headers").and_then(|v| v.as_array()) {
            for entry in headers {
                if let (Some(name), Some(value)) = (
                    entry.get("name").and_then(|v| v.as_str()),
                    entry.get("value").and_then(|v| v.as_str()),
                ) {
                    map.insert(name.to_lowercase(), value.to_string());
                }
            }
        }
        map
    }

    /// Returns the raw base64-encoded post data from the initializer, or `None`.
    fn post_data_b64(&self) -> Option<&str> {
        self.initializer().get("postData").and_then(|v| v.as_str())
    }

    /// Returns the request body (POST data) as bytes, or `None` if there is no body.
    ///
    /// The Playwright protocol sends `postData` as a base64-encoded string.
    /// This method decodes it to raw bytes.
    ///
    /// This is a local read and does not require an RPC call.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-post-data-buffer>
    pub fn post_data_buffer(&self) -> Option<Vec<u8>> {
        let b64 = self.post_data_b64()?;
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(b64).ok()
    }

    /// Returns the request body (POST data) as a UTF-8 string, or `None` if there is no body.
    ///
    /// The Playwright protocol sends `postData` as a base64-encoded string.
    /// This method decodes the base64 and then converts the bytes to a UTF-8 string.
    ///
    /// This is a local read and does not require an RPC call.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-post-data>
    pub fn post_data(&self) -> Option<String> {
        let bytes = self.post_data_buffer()?;
        String::from_utf8(bytes).ok()
    }

    /// Parses the POST data as JSON and deserializes into the target type `T`.
    ///
    /// Returns `None` if the request has no POST data, or `Some(Err(...))` if the
    /// JSON parsing fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-post-data-json>
    pub fn post_data_json<T: DeserializeOwned>(&self) -> Option<Result<T>> {
        let data = self.post_data()?;
        Some(serde_json::from_str(&data).map_err(|e| {
            crate::error::Error::ProtocolError(format!(
                "Failed to parse request post data as JSON: {}",
                e
            ))
        }))
    }

    /// Returns the error text if the request failed, or `None` for successful requests.
    ///
    /// The failure text is set when the `requestFailed` browser event fires for this
    /// request. Use `page.on_request_failed()` to capture failed requests and then
    /// call this method to get the error reason.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-failure>
    pub fn failure(&self) -> Option<String> {
        self.failure_text.lock().unwrap().clone()
    }

    /// Sets the failure text. Called by the dispatcher when a `requestFailed` event
    /// arrives for this request.
    pub(crate) fn set_failure_text(&self, text: String) {
        *self.failure_text.lock().unwrap() = Some(text);
    }

    /// Sets the timing data. Called by the dispatcher when a `requestFinished` event
    /// arrives and timing data is extracted from the associated Response's initializer.
    pub(crate) fn set_timing(&self, timing_val: Value) {
        *self.timing.lock().unwrap() = Some(timing_val);
    }

    /// Returns all request headers as name-value pairs, preserving duplicates.
    ///
    /// Sends a `"rawRequestHeaders"` RPC call to the Playwright server which returns
    /// the complete list of headers as sent over the wire, including headers added by
    /// the browser (e.g., `accept-encoding`, `accept-language`).
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call to the server fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-headers-array>
    pub async fn headers_array(&self) -> Result<Vec<HeaderEntry>> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct RawHeadersResponse {
            headers: Vec<HeaderEntryRaw>,
        }

        #[derive(Deserialize)]
        struct HeaderEntryRaw {
            name: String,
            value: String,
        }

        let result: RawHeadersResponse = self
            .channel()
            .send("rawRequestHeaders", serde_json::json!({}))
            .await?;

        Ok(result
            .headers
            .into_iter()
            .map(|h| HeaderEntry {
                name: h.name,
                value: h.value,
            })
            .collect())
    }

    /// Returns all request headers as a `HashMap<String, String>` with lowercased keys.
    ///
    /// When multiple headers have the same name, their values are joined with `\n`
    /// (matching Playwright's behavior).
    ///
    /// Sends a `"rawRequestHeaders"` RPC call to the Playwright server.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call to the server fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-all-headers>
    pub async fn all_headers(&self) -> Result<HashMap<String, String>> {
        let entries = self.headers_array().await?;
        let mut map: HashMap<String, String> = HashMap::new();
        for entry in entries {
            let key = entry.name.to_lowercase();
            map.entry(key)
                .and_modify(|existing| {
                    existing.push('\n');
                    existing.push_str(&entry.value);
                })
                .or_insert(entry.value);
        }
        Ok(map)
    }

    /// Returns the value of the specified header (case-insensitive), or `None` if not found.
    ///
    /// Uses [`all_headers()`](Self::all_headers) internally, so it sends a
    /// `"rawRequestHeaders"` RPC call to the Playwright server.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call to the server fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-header-value>
    pub async fn header_value(&self, name: &str) -> Result<Option<String>> {
        let all = self.all_headers().await?;
        Ok(all.get(&name.to_lowercase()).cloned())
    }

    /// Returns timing information for the request.
    ///
    /// The timing data is sourced from the associated Response's initializer when the
    /// `requestFinished` event fires. This method should be called from within a
    /// `page.on_request_finished()` handler or after it has fired.
    ///
    /// Fields use `-1` to indicate that a timing phase was not reached or is
    /// unavailable for a given request.
    ///
    /// # Errors
    ///
    /// Returns an error if timing data is not yet available (e.g., called before
    /// `requestFinished` fires, or for a request that has not completed successfully).
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-timing>
    pub async fn timing(&self) -> Result<ResourceTiming> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawTiming {
            start_time: Option<f64>,
            domain_lookup_start: Option<f64>,
            domain_lookup_end: Option<f64>,
            connect_start: Option<f64>,
            connect_end: Option<f64>,
            secure_connection_start: Option<f64>,
            request_start: Option<f64>,
            response_start: Option<f64>,
            response_end: Option<f64>,
        }

        let timing_val = self.timing.lock().unwrap().clone().ok_or_else(|| {
            crate::error::Error::ProtocolError(
                "Request timing is not yet available. Call timing() from \
                     on_request_finished() or after it has fired."
                    .to_string(),
            )
        })?;

        let raw: RawTiming = serde_json::from_value(timing_val).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to parse timing data: {}", e))
        })?;

        Ok(ResourceTiming {
            start_time: raw.start_time.unwrap_or(-1.0),
            domain_lookup_start: raw.domain_lookup_start.unwrap_or(-1.0),
            domain_lookup_end: raw.domain_lookup_end.unwrap_or(-1.0),
            connect_start: raw.connect_start.unwrap_or(-1.0),
            connect_end: raw.connect_end.unwrap_or(-1.0),
            secure_connection_start: raw.secure_connection_start.unwrap_or(-1.0),
            request_start: raw.request_start.unwrap_or(-1.0),
            response_start: raw.response_start.unwrap_or(-1.0),
            response_end: raw.response_end.unwrap_or(-1.0),
        })
    }
}

/// Resource timing information for an HTTP request.
///
/// All time values are in milliseconds relative to the navigation start.
/// A value of `-1` indicates the timing phase was not reached.
///
/// See: <https://playwright.dev/docs/api/class-request#request-timing>
#[derive(Debug, Clone)]
pub struct ResourceTiming {
    /// Request start time in milliseconds since epoch.
    pub start_time: f64,
    /// Time immediately before the browser starts the domain name lookup
    /// for the resource. The value is given in milliseconds relative to
    /// `startTime`, -1 if not available.
    pub domain_lookup_start: f64,
    /// Time immediately after the browser starts the domain name lookup
    /// for the resource. The value is given in milliseconds relative to
    /// `startTime`, -1 if not available.
    pub domain_lookup_end: f64,
    /// Time immediately before the user agent starts establishing the
    /// connection to the server to retrieve the resource.
    pub connect_start: f64,
    /// Time immediately after the browser starts the handshake process
    /// to secure the current connection.
    pub secure_connection_start: f64,
    /// Time immediately after the browser finishes establishing the connection
    /// to the server to retrieve the resource.
    pub connect_end: f64,
    /// Time immediately before the browser starts requesting the resource from
    /// the server, cache, or local resource.
    pub request_start: f64,
    /// Time immediately after the browser starts requesting the resource from
    /// the server, cache, or local resource.
    pub response_start: f64,
    /// Time immediately after the browser receives the last byte of the resource
    /// or immediately before the transport connection is closed, whichever comes first.
    pub response_end: f64,
}

impl ChannelOwner for Request {
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
        // Request events will be handled in future phases
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("guid", &self.guid())
            .finish()
    }
}
