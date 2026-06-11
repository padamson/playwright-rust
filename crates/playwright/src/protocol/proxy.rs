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
#[non_exhaustive]
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

impl ProxySettings {
    /// Proxy all traffic through the given server (e.g. "http://host:3128").
    pub fn new(server: impl Into<String>) -> Self {
        Self {
            server: server.into(),
            bypass: None,
            username: None,
            password: None,
        }
    }
    /// Comma-separated domains to bypass the proxy for.
    pub fn bypass(mut self, bypass: impl Into<String>) -> Self {
        self.bypass = Some(bypass.into());
        self
    }
    /// Proxy auth username.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }
    /// Proxy auth password.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }
}
