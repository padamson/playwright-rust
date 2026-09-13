use crate::server::connection::downcast_parent;

/// Response from navigation operations.
///
/// Returned from `page.goto()`, `page.reload()`, `page.go_back()`, and similar
/// navigation methods. Provides access to the HTTP response status, headers, and body.
///
/// See: <https://playwright.dev/docs/api/class-response>
#[derive(Clone)]
pub struct Response {
    url: String,
    status: u16,
    status_text: String,
    ok: bool,
    headers: std::collections::HashMap<String, String>,
    /// Reference to the backing channel owner for RPC calls (body, rawHeaders, etc.)
    /// Stored as the generic trait object so it can be downcast to ResponseObject when needed.
    response_channel_owner: Option<std::sync::Arc<dyn crate::server::channel_owner::ChannelOwner>>,
}

impl Response {
    /// Creates a new Response from protocol data.
    ///
    /// This is used internally when constructing a Response from the protocol
    /// initializer (e.g., after `goto` or `reload`).
    pub(crate) fn new(
        url: String,
        status: u16,
        status_text: String,
        headers: std::collections::HashMap<String, String>,
        response_channel_owner: Option<
            std::sync::Arc<dyn crate::server::channel_owner::ChannelOwner>,
        >,
    ) -> Self {
        Self {
            url,
            status,
            status_text,
            ok: (200..300).contains(&status),
            headers,
            response_channel_owner,
        }
    }
}

impl Response {
    /// Returns the URL of the response.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-url>
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the HTTP status code.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-status>
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Returns the HTTP status text.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-status-text>
    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    /// Returns whether the response was successful (status 200-299).
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-ok>
    pub fn ok(&self) -> bool {
        self.ok
    }

    /// Returns the response headers as a HashMap.
    ///
    /// Note: these are the headers from the protocol initializer. For the full
    /// raw headers (including duplicates), use `headers_array()` or `all_headers()`.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-headers>
    pub fn headers(&self) -> &std::collections::HashMap<String, String> {
        &self.headers
    }

    /// Returns the [`Request`](crate::protocol::Request) that triggered this response.
    ///
    /// Navigates the protocol object hierarchy: ResponseObject → parent (Request).
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-request>
    pub fn request(&self) -> Option<crate::protocol::Request> {
        let owner = self.response_channel_owner.as_ref()?;
        downcast_parent::<crate::protocol::Request>(&**owner)
    }

    /// Returns the [`Frame`](crate::protocol::Frame) that initiated the request for this response.
    ///
    /// Navigates the protocol object hierarchy: ResponseObject → Request → Frame.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-frame>
    pub fn frame(&self) -> Option<crate::protocol::Frame> {
        let request = self.request()?;
        request.frame()
    }

    /// Returns the backing `ResponseObject`, or an error if unavailable.
    pub(crate) fn response_object(&self) -> crate::error::Result<crate::protocol::ResponseObject> {
        let arc = self.response_channel_owner.as_ref().ok_or_else(|| {
            crate::error::Error::ProtocolError(
                "Response has no backing protocol object".to_string(),
            )
        })?;
        arc.as_any()
            .downcast_ref::<crate::protocol::ResponseObject>()
            .cloned()
            .ok_or_else(|| crate::error::Error::TypeMismatch {
                guid: arc.guid().to_string(),
                expected: "ResponseObject".to_string(),
                actual: arc.type_name().to_string(),
            })
    }

    /// Returns TLS/SSL security details for HTTPS connections, or `None` for HTTP.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-security-details>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url()))]
    pub async fn security_details(
        &self,
    ) -> crate::error::Result<Option<crate::protocol::response::SecurityDetails>> {
        self.response_object()?.security_details().await
    }

    /// Returns the server's IP address and port, or `None`.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-server-addr>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url()))]
    pub async fn server_addr(
        &self,
    ) -> crate::error::Result<Option<crate::protocol::response::RemoteAddr>> {
        self.response_object()?.server_addr().await
    }

    /// Waits for this response to finish loading.
    ///
    /// For responses obtained from navigation methods (`goto`, `reload`), the response
    /// is already finished when returned. For responses from `on_response` handlers,
    /// the body may still be loading.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-finished>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url()))]
    pub async fn finished(&self) -> crate::error::Result<()> {
        // The Playwright protocol dispatches `requestFinished` as a separate event
        // rather than exposing a `finished` RPC method on Response.
        // For responses from goto/reload, the response is already complete.
        // TODO: For on_response handlers, implement proper waiting via requestFinished event.
        Ok(())
    }

    /// Returns the HTTP version used by this response (e.g. `"HTTP/1.1"` or `"HTTP/2.0"`).
    ///
    /// Makes an RPC call to the Playwright server.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No backing protocol object is available (edge case)
    /// - The RPC call to the server fails
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-http-version>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url(), version = tracing::field::Empty))]
    pub async fn http_version(&self) -> crate::error::Result<String> {
        let v = self.response_object()?.http_version().await?;
        tracing::Span::current().record("version", &v);
        Ok(v)
    }

    /// Returns the response body as raw bytes.
    ///
    /// Makes an RPC call to the Playwright server to fetch the response body.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No backing protocol object is available (edge case)
    /// - The RPC call to the server fails
    /// - The base64 response cannot be decoded
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-body>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url(), bytes_len = tracing::field::Empty))]
    pub async fn body(&self) -> crate::error::Result<Vec<u8>> {
        let bytes = self.response_object()?.body().await?;
        tracing::Span::current().record("bytes_len", bytes.len());
        Ok(bytes)
    }

    /// Returns the response body as a UTF-8 string.
    ///
    /// Calls `body()` then converts bytes to a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `body()` fails
    /// - The body is not valid UTF-8
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-text>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url()))]
    pub async fn text(&self) -> crate::error::Result<String> {
        let bytes = self.body().await?;
        String::from_utf8(bytes).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Response body is not valid UTF-8: {}", e))
        })
    }

    /// Parses the response body as JSON and deserializes it into type `T`.
    ///
    /// Calls `text()` then uses `serde_json` to deserialize the body.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `text()` fails
    /// - The body is not valid JSON or doesn't match the expected type
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-json>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url()))]
    pub async fn json<T: serde::de::DeserializeOwned>(&self) -> crate::error::Result<T> {
        let text = self.text().await?;
        serde_json::from_str(&text).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to parse response JSON: {}", e))
        })
    }

    /// Returns all response headers as name-value pairs, preserving duplicates.
    ///
    /// Makes an RPC call for `"rawHeaders"` which returns the complete header list.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No backing protocol object is available (edge case)
    /// - The RPC call to the server fails
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-headers-array>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url()))]
    pub async fn headers_array(
        &self,
    ) -> crate::error::Result<Vec<crate::protocol::response::HeaderEntry>> {
        self.response_object()?.raw_headers().await
    }

    /// Returns all response headers merged into a HashMap with lowercase keys.
    ///
    /// When multiple headers have the same name, their values are joined with `, `.
    /// This matches the behavior of `response.allHeaders()` in other Playwright bindings.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No backing protocol object is available (edge case)
    /// - The RPC call to the server fails
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-all-headers>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url()))]
    pub async fn all_headers(
        &self,
    ) -> crate::error::Result<std::collections::HashMap<String, String>> {
        Ok(crate::protocol::route_params::merge_headers(
            self.headers_array()
                .await?
                .into_iter()
                .map(|entry| (entry.name, entry.value)),
            Some(", "),
        ))
    }

    /// Returns the value for a single response header, or `None` if not present.
    ///
    /// The lookup is case-insensitive.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No backing protocol object is available (edge case)
    /// - The RPC call to the server fails
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-header-value>
    /// Returns the value for a single response header, or `None` if not present.
    ///
    /// The lookup is case-insensitive. When multiple headers share the same name,
    /// their values are joined with `, ` (matching Playwright's behavior).
    ///
    /// Uses the raw headers from the server for accurate results.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `headers_array()` RPC call fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-header-value>
    #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url(), name = %name))]
    pub async fn header_value(&self, name: &str) -> crate::error::Result<Option<String>> {
        Ok(self.all_headers().await?.get(&name.to_lowercase()).cloned())
    }
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("url", &self.url)
            .field("status", &self.status)
            .field("status_text", &self.status_text)
            .field("ok", &self.ok)
            .finish_non_exhaustive()
    }
}
