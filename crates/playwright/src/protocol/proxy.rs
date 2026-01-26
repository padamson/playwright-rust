//! Network proxy settings
//!
//! This module defines the [`ProxySettings`] struct which encapsulates
//! the configuration for proxies used for network requests.
//!
//! Proxy settings can be applied at both the browser launch level
//! ([`LaunchOptions`](crate::api::LaunchOptions)) and the browser context level
//! ([`BrowserContextOptions`](crate::protocol::BrowserContextOptions)).
//!
//! See: <https://playwright.dev/docs/api/class-browser#browser-new-context>

use serde::{Deserialize, Serialize};

/// Network proxy settings for browser contexts and browser launches.
///
/// HTTP and SOCKS proxies are supported. Example proxy URLs:
/// - `http://myproxy.com:3128`
/// - `socks5://myproxy.com:3128`
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::ProxySettings;
///
/// let proxy = ProxySettings {
///     server: "http://proxy.example.com:8080".to_string(),
///     bypass: Some(".example.com, chromium.org".to_string()),
///     username: Some("user".to_string()),
///     password: Some("secret".to_string()),
/// };
/// ```
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySettings {
    /// Proxy server URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
    pub server: String,

    /// Comma-separated domains to bypass proxy (e.g., ".example.com, chromium.org")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bypass: Option<String>,

    /// Proxy username for HTTP proxy authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Proxy password for HTTP proxy authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}
