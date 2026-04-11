// Response protocol object
//
// Represents an HTTP response from navigation operations.
// Response objects are created by the server when Frame.goto() or similar navigation
// methods complete successfully.

use crate::error::Result;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// TLS/SSL security details for an HTTPS response.
///
/// All fields are optional — the server provides what's available.
///
/// See: <https://playwright.dev/docs/api/class-response#response-security-details>
#[derive(Debug, Clone)]
pub struct SecurityDetails {
    /// Certificate issuer name.
    pub issuer: Option<String>,
    /// TLS protocol version (e.g., "TLS 1.3").
    pub protocol: Option<String>,
    /// Certificate subject name.
    pub subject_name: Option<String>,
    /// Unix timestamp (seconds) when the certificate becomes valid.
    pub valid_from: Option<f64>,
    /// Unix timestamp (seconds) when the certificate expires.
    pub valid_to: Option<f64>,
}

/// Remote server address (IP and port).
///
/// See: <https://playwright.dev/docs/api/class-response#response-server-addr>
#[derive(Debug, Clone)]
pub struct RemoteAddr {
    /// Server IP address.
    pub ip_address: String,
    /// Server port.
    pub port: u16,
}

/// Resource size information for a request/response pair.
///
/// See: <https://playwright.dev/docs/api/class-request#request-sizes>
#[derive(Debug, Clone)]
pub struct RequestSizes {
    /// Size of the request body in bytes. Set to 0 if there was no body.
    pub request_body_size: i64,
    /// Total number of bytes from the start of the HTTP request message
    /// until (and including) the double CRLF before the body.
    pub request_headers_size: i64,
    /// Size of the received response body in bytes.
    pub response_body_size: i64,
    /// Total number of bytes from the start of the HTTP response message
    /// until (and including) the double CRLF before the body.
    pub response_headers_size: i64,
}

/// A single HTTP header entry with a name and value.
///
/// Used by `Response::headers_array()` to return all headers preserving duplicates.
///
/// See: <https://playwright.dev/docs/api/class-response#response-headers-array>
#[derive(Debug, Clone)]
pub struct HeaderEntry {
    /// Header name (lowercase)
    pub name: String,
    /// Header value
    pub value: String,
}

/// Response represents an HTTP response from a navigation operation.
///
/// Response objects are not created directly - they are returned from
/// navigation methods like page.goto() or page.reload().
///
/// See: <https://playwright.dev/docs/api/class-response>
#[derive(Clone)]
pub struct ResponseObject {
    base: ChannelOwnerImpl,
}

impl ResponseObject {
    /// Creates a new Response from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Response object.
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

    /// Returns the status code of the response (e.g., 200 for a success).
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-status>
    pub fn status(&self) -> u16 {
        self.initializer()
            .get("status")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u16
    }

    /// Returns the status text of the response (e.g. usually an "OK" for a success).
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-status-text>
    pub fn status_text(&self) -> &str {
        self.initializer()
            .get("statusText")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Returns the URL of the response.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-url>
    pub fn url(&self) -> &str {
        self.initializer()
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Returns the response body as bytes.
    ///
    /// Sends a `"body"` RPC call to the Playwright server, which returns the body
    /// as a base64-encoded binary string.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-body>
    pub async fn body(&self) -> Result<Vec<u8>> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct BodyResponse {
            binary: String,
        }

        let result: BodyResponse = self.channel().send("body", serde_json::json!({})).await?;

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&result.binary)
            .map_err(|e| {
                crate::error::Error::ProtocolError(format!(
                    "Failed to decode response body from base64: {}",
                    e
                ))
            })?;
        Ok(bytes)
    }

    /// Returns TLS/SSL security details for HTTPS connections, or `None` for HTTP.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-security-details>
    pub async fn security_details(&self) -> Result<Option<SecurityDetails>> {
        let result: serde_json::Value = self
            .channel()
            .send("securityDetails", serde_json::json!({}))
            .await?;

        let value = result.get("value");
        match value {
            Some(v) if v.as_object().is_some_and(|obj| !obj.is_empty()) => {
                Ok(Some(SecurityDetails {
                    issuer: v.get("issuer").and_then(|v| v.as_str()).map(String::from),
                    protocol: v.get("protocol").and_then(|v| v.as_str()).map(String::from),
                    subject_name: v
                        .get("subjectName")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    valid_from: v.get("validFrom").and_then(|v| v.as_f64()),
                    valid_to: v.get("validTo").and_then(|v| v.as_f64()),
                }))
            }
            _ => Ok(None),
        }
    }

    /// Returns the server's IP address and port for this response, or `None`.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-server-addr>
    pub async fn server_addr(&self) -> Result<Option<RemoteAddr>> {
        let result: serde_json::Value = self
            .channel()
            .send("serverAddr", serde_json::json!({}))
            .await?;

        let value = result.get("value");
        match value {
            Some(v) if !v.is_null() => {
                let ip_address = v
                    .get("ipAddress")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let port = v.get("port").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
                Ok(Some(RemoteAddr { ip_address, port }))
            }
            _ => Ok(None),
        }
    }

    /// Returns resource size information for this response.
    ///
    /// See: <https://playwright.dev/docs/api/class-request#request-sizes>
    pub async fn sizes(&self) -> Result<RequestSizes> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SizesRaw {
            request_body_size: i64,
            request_headers_size: i64,
            response_body_size: i64,
            response_headers_size: i64,
        }

        #[derive(Deserialize)]
        struct RpcResult {
            sizes: SizesRaw,
        }

        let result: RpcResult = self.channel().send("sizes", serde_json::json!({})).await?;

        Ok(RequestSizes {
            request_body_size: result.sizes.request_body_size,
            request_headers_size: result.sizes.request_headers_size,
            response_body_size: result.sizes.response_body_size,
            response_headers_size: result.sizes.response_headers_size,
        })
    }

    /// Returns the raw response headers as name-value pairs (preserving duplicates).
    ///
    /// Sends a `"rawResponseHeaders"` RPC call to the Playwright server.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-headers-array>
    pub async fn raw_headers(&self) -> Result<Vec<HeaderEntry>> {
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
            .send("rawResponseHeaders", serde_json::json!({}))
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
}

impl ChannelOwner for ResponseObject {
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
        // Response objects don't have events
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for ResponseObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResponseObject")
            .field("guid", &self.guid())
            .finish()
    }
}
