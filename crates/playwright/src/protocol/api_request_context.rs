// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// APIRequestContext protocol object
//
// Enables performing HTTP requests without a browser, and is also used
// by Route.fetch() to perform the actual network request before modification.
//
// See: https://playwright.dev/docs/api/class-apirequestcontext

use crate::error::Result;
use crate::protocol::route::FetchOptions;
use crate::protocol::route::FetchResponse;
use crate::server::channel::Channel;
use crate::server::channel_owner::{
    ChannelOwner, ChannelOwnerImpl, DisposeReason, ParentOrConnection,
};
use crate::server::connection::ConnectionLike;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// APIRequestContext provides methods for making HTTP requests.
///
/// This is the Playwright protocol object that performs actual HTTP operations.
/// It is created automatically for each BrowserContext and can be accessed
/// via `BrowserContext::request()`.
///
/// Used internally by `Route::fetch()` to perform the actual network request.
///
/// See: <https://playwright.dev/docs/api/class-apirequestcontext>
#[derive(Clone)]
pub struct APIRequestContext {
    base: ChannelOwnerImpl,
}

impl APIRequestContext {
    pub fn new(
        parent: ParentOrConnection,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        Ok(Self {
            base: ChannelOwnerImpl::new(parent, type_name, guid, initializer),
        })
    }

    /// Sends a GET request.
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequestcontext#api-request-context-get>
    pub async fn get(&self, url: &str, options: Option<FetchOptions>) -> Result<APIResponse> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("GET".to_string());
        self.fetch(url, Some(opts)).await
    }

    /// Sends a POST request.
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequestcontext#api-request-context-post>
    pub async fn post(&self, url: &str, options: Option<FetchOptions>) -> Result<APIResponse> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("POST".to_string());
        self.fetch(url, Some(opts)).await
    }

    /// Sends a PUT request.
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequestcontext#api-request-context-put>
    pub async fn put(&self, url: &str, options: Option<FetchOptions>) -> Result<APIResponse> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("PUT".to_string());
        self.fetch(url, Some(opts)).await
    }

    /// Sends a DELETE request.
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequestcontext#api-request-context-delete>
    pub async fn delete(&self, url: &str, options: Option<FetchOptions>) -> Result<APIResponse> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("DELETE".to_string());
        self.fetch(url, Some(opts)).await
    }

    /// Sends a PATCH request.
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequestcontext#api-request-context-patch>
    pub async fn patch(&self, url: &str, options: Option<FetchOptions>) -> Result<APIResponse> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("PATCH".to_string());
        self.fetch(url, Some(opts)).await
    }

    /// Sends a HEAD request.
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequestcontext#api-request-context-head>
    pub async fn head(&self, url: &str, options: Option<FetchOptions>) -> Result<APIResponse> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("HEAD".to_string());
        self.fetch(url, Some(opts)).await
    }

    /// Sends a fetch request with the given options, returning an `APIResponse`.
    ///
    /// This is the public-facing fetch method that returns a lazy `APIResponse`.
    /// The response body is not fetched until `body()`, `text()`, or `json()` is called.
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequestcontext#api-request-context-fetch>
    pub async fn fetch(&self, url: &str, options: Option<FetchOptions>) -> Result<APIResponse> {
        let opts = options.unwrap_or_default();

        let mut params = json!({
            "url": url,
            "timeout": opts.timeout.unwrap_or(crate::DEFAULT_TIMEOUT_MS)
        });

        if let Some(method) = opts.method {
            params["method"] = json!(method);
        }
        if let Some(headers) = opts.headers {
            let headers_array: Vec<Value> = headers
                .into_iter()
                .map(|(name, value)| json!({"name": name, "value": value}))
                .collect();
            params["headers"] = json!(headers_array);
        }
        if let Some(post_data) = opts.post_data {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(post_data.as_bytes());
            params["postData"] = json!(encoded);
        } else if let Some(post_data_bytes) = opts.post_data_bytes {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&post_data_bytes);
            params["postData"] = json!(encoded);
        }
        if let Some(max_redirects) = opts.max_redirects {
            params["maxRedirects"] = json!(max_redirects);
        }
        if let Some(max_retries) = opts.max_retries {
            params["maxRetries"] = json!(max_retries);
        }

        #[derive(serde::Deserialize)]
        struct FetchResult {
            response: ApiResponseData,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ApiResponseData {
            fetch_uid: String,
            url: String,
            status: u16,
            status_text: String,
            headers: Vec<HeaderEntry>,
        }

        #[derive(serde::Deserialize)]
        struct HeaderEntry {
            name: String,
            value: String,
        }

        let result: FetchResult = self.base.channel().send("fetch", params).await?;

        let headers: HashMap<String, String> = result
            .response
            .headers
            .into_iter()
            .map(|h| (h.name, h.value))
            .collect();

        Ok(APIResponse {
            context: self.clone(),
            url: result.response.url,
            status: result.response.status,
            status_text: result.response.status_text,
            headers,
            fetch_uid: result.response.fetch_uid,
        })
    }

    /// Disposes this `APIRequestContext`, freeing server resources.
    ///
    /// After calling `dispose()`, the context cannot be used for further requests.
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequestcontext#api-request-context-dispose>
    pub async fn dispose(&self) -> Result<()> {
        self.base
            .channel()
            .send_no_result("dispose", json!({}))
            .await
    }

    /// Performs an HTTP fetch request and returns the response.
    ///
    /// This is the internal method used by `Route::fetch()`. It sends the request
    /// via the Playwright server and returns the response with headers and body.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch
    /// * `options` - Optional parameters to customize the request
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequestcontext#api-request-context-fetch>
    pub(crate) async fn inner_fetch(
        &self,
        url: &str,
        options: Option<InnerFetchOptions>,
    ) -> Result<FetchResponse> {
        let opts = options.unwrap_or_default();

        let mut params = json!({
            "url": url,
            "timeout": opts.timeout.unwrap_or(crate::DEFAULT_TIMEOUT_MS)
        });

        if let Some(method) = opts.method {
            params["method"] = json!(method);
        }
        if let Some(headers) = opts.headers {
            let headers_array: Vec<Value> = headers
                .into_iter()
                .map(|(name, value)| json!({"name": name, "value": value}))
                .collect();
            params["headers"] = json!(headers_array);
        }
        if let Some(post_data) = opts.post_data {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(post_data.as_bytes());
            params["postData"] = json!(encoded);
        }
        if let Some(post_data_bytes) = opts.post_data_bytes {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&post_data_bytes);
            params["postData"] = json!(encoded);
        }
        if let Some(max_redirects) = opts.max_redirects {
            params["maxRedirects"] = json!(max_redirects);
        }
        if let Some(max_retries) = opts.max_retries {
            params["maxRetries"] = json!(max_retries);
        }

        // Call the fetch command on APIRequestContext channel
        #[derive(serde::Deserialize)]
        struct FetchResult {
            response: ApiResponseData,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ApiResponseData {
            fetch_uid: String,
            #[allow(dead_code)]
            url: String,
            status: u16,
            status_text: String,
            headers: Vec<HeaderEntry>,
        }

        #[derive(serde::Deserialize)]
        struct HeaderEntry {
            name: String,
            value: String,
        }

        let result: FetchResult = self.base.channel().send("fetch", params).await?;

        // Now fetch the response body using fetchResponseBody
        let body = self.fetch_response_body(&result.response.fetch_uid).await?;

        // Dispose the API response to free server resources
        let _ = self.dispose_api_response(&result.response.fetch_uid).await;

        Ok(FetchResponse {
            status: result.response.status,
            status_text: result.response.status_text,
            headers: result
                .response
                .headers
                .into_iter()
                .map(|h| (h.name, h.value))
                .collect(),
            body,
        })
    }

    /// Fetches the response body for a given fetch UID.
    async fn fetch_response_body(&self, fetch_uid: &str) -> Result<Vec<u8>> {
        #[derive(serde::Deserialize)]
        struct BodyResult {
            #[serde(default)]
            binary: Option<String>,
        }

        let result: BodyResult = self
            .base
            .channel()
            .send("fetchResponseBody", json!({ "fetchUid": fetch_uid }))
            .await?;

        match result.binary {
            Some(encoded) if !encoded.is_empty() => {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD
                    .decode(&encoded)
                    .map_err(|e| {
                        crate::error::Error::ProtocolError(format!(
                            "Failed to decode response body: {}",
                            e
                        ))
                    })
            }
            _ => Ok(vec![]),
        }
    }

    /// Disposes an API response to free server resources.
    async fn dispose_api_response(&self, fetch_uid: &str) -> Result<()> {
        self.base
            .channel()
            .send_no_result("disposeAPIResponse", json!({ "fetchUid": fetch_uid }))
            .await
    }
}

/// A lazy HTTP response returned by `APIRequestContext` methods.
///
/// Unlike [`crate::protocol::route::FetchResponse`] (which eagerly fetches the body),
/// `APIResponse` holds a `fetch_uid` and fetches the body on demand.
///
/// See: <https://playwright.dev/docs/api/class-apiresponse>
#[derive(Clone)]
pub struct APIResponse {
    context: APIRequestContext,
    url: String,
    status: u16,
    status_text: String,
    headers: HashMap<String, String>,
    fetch_uid: String,
}

impl APIResponse {
    /// Returns the URL of the response.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the HTTP status code.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Returns the HTTP status text (e.g., "OK", "Not Found").
    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    /// Returns `true` if the status code is in the 200–299 range.
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Returns the response headers as a `HashMap<String, String>`.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Fetches and returns the response body as bytes.
    ///
    /// See: <https://playwright.dev/docs/api/class-apiresponse#api-response-body>
    pub async fn body(&self) -> Result<Vec<u8>> {
        self.context.fetch_response_body(&self.fetch_uid).await
    }

    /// Fetches and returns the response body as a UTF-8 string.
    ///
    /// See: <https://playwright.dev/docs/api/class-apiresponse#api-response-text>
    pub async fn text(&self) -> Result<String> {
        let bytes = self.body().await?;
        String::from_utf8(bytes).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Response body is not valid UTF-8: {}", e))
        })
    }

    /// Fetches the response body and deserializes it as JSON.
    ///
    /// See: <https://playwright.dev/docs/api/class-apiresponse#api-response-json>
    pub async fn json<T: DeserializeOwned>(&self) -> Result<T> {
        let bytes = self.body().await?;
        serde_json::from_slice(&bytes).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to parse response JSON: {}", e))
        })
    }

    /// Disposes this response, freeing server-side resources for the response body.
    ///
    /// See: <https://playwright.dev/docs/api/class-apiresponse#api-response-dispose>
    pub async fn dispose(&self) -> Result<()> {
        self.context.dispose_api_response(&self.fetch_uid).await
    }
}

impl std::fmt::Debug for APIResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("APIResponse")
            .field("url", &self.url)
            .field("status", &self.status)
            .field("status_text", &self.status_text)
            .finish()
    }
}

/// Options for creating a new `APIRequestContext` via `APIRequest::new_context()`.
///
/// See: <https://playwright.dev/docs/api/class-apirequest#api-request-new-context>
#[derive(Debug, Clone, Default)]
pub struct APIRequestContextOptions {
    /// Base URL for all relative requests made with this context.
    pub base_url: Option<String>,
    /// Extra HTTP headers to be sent with every request.
    pub extra_http_headers: Option<HashMap<String, String>>,
    /// Whether to ignore HTTPS errors when making requests.
    pub ignore_https_errors: Option<bool>,
    /// User agent string to send with requests.
    pub user_agent: Option<String>,
    /// Default timeout for fetch operations in milliseconds.
    pub timeout: Option<f64>,
}

/// Factory for creating standalone `APIRequestContext` instances.
///
/// Obtained via `playwright.request()`. Use `new_context()` to create a context
/// for making HTTP requests outside of a browser page.
///
/// `APIRequest` intentionally holds only the channel and connection reference,
/// NOT a `Playwright` clone. Holding a `Playwright` clone would trigger the
/// server shutdown Drop impl when the temporary `APIRequest` is dropped.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::Playwright;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///
///     let ctx = playwright.request().new_context(None).await?;
///     let response = ctx.get("https://example.com/api/data", None).await?;
///     assert!(response.ok());
///     let body = response.text().await?;
///
///     ctx.dispose().await?;
///     playwright.shutdown().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-apirequest>
pub struct APIRequest {
    channel: crate::server::channel::Channel,
    connection: Arc<dyn ConnectionLike>,
}

impl APIRequest {
    pub(crate) fn new(
        channel: crate::server::channel::Channel,
        connection: Arc<dyn ConnectionLike>,
    ) -> Self {
        Self {
            channel,
            connection,
        }
    }

    /// Creates a new `APIRequestContext` for making HTTP requests.
    ///
    /// # Arguments
    ///
    /// * `options` — Optional configuration for the new context
    ///
    /// See: <https://playwright.dev/docs/api/class-apirequest#api-request-new-context>
    pub async fn new_context(
        &self,
        options: Option<APIRequestContextOptions>,
    ) -> Result<APIRequestContext> {
        use crate::server::connection::ConnectionExt;

        let mut params = json!({});

        if let Some(opts) = options {
            if let Some(base_url) = opts.base_url {
                params["baseURL"] = json!(base_url);
            }
            if let Some(headers) = opts.extra_http_headers {
                let arr: Vec<Value> = headers
                    .into_iter()
                    .map(|(name, value)| json!({"name": name, "value": value}))
                    .collect();
                params["extraHTTPHeaders"] = json!(arr);
            }
            if let Some(ignore) = opts.ignore_https_errors {
                params["ignoreHTTPSErrors"] = json!(ignore);
            }
            if let Some(ua) = opts.user_agent {
                params["userAgent"] = json!(ua);
            }
            if let Some(timeout) = opts.timeout {
                params["timeout"] = json!(timeout);
            }
        }

        #[derive(serde::Deserialize)]
        struct NewRequestResult {
            request: GuidRef,
        }

        #[derive(serde::Deserialize)]
        struct GuidRef {
            guid: String,
        }

        let result: NewRequestResult = self.channel.send("newRequest", params).await?;

        self.connection
            .get_typed::<APIRequestContext>(&result.request.guid)
            .await
    }
}

impl std::fmt::Debug for APIRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("APIRequest").finish()
    }
}

/// Options for APIRequestContext.inner_fetch()
#[derive(Debug, Clone, Default)]
pub(crate) struct InnerFetchOptions {
    pub method: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub post_data: Option<String>,
    pub post_data_bytes: Option<Vec<u8>>,
    pub max_redirects: Option<u32>,
    pub max_retries: Option<u32>,
    pub timeout: Option<f64>,
}

impl ChannelOwner for APIRequestContext {
    fn guid(&self) -> &str {
        self.base.guid()
    }

    fn type_name(&self) -> &str {
        self.base.type_name()
    }

    fn parent(&self) -> Option<Arc<dyn ChannelOwner>> {
        self.base.parent()
    }

    fn connection(&self) -> Arc<dyn ConnectionLike> {
        self.base.connection()
    }

    fn initializer(&self) -> &Value {
        self.base.initializer()
    }

    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    fn dispose(&self, reason: DisposeReason) {
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
        self.base.on_event(method, params)
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for APIRequestContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("APIRequestContext")
            .field("guid", &self.guid())
            .finish()
    }
}
