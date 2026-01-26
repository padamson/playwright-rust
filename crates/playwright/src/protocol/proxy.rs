// Network proxy settings
//
// This module defines the ProxySettings struct which encapsulates
// the configuration for proxies used for network requests.

use serde::{Deserialize, Serialize};

/// Network proxy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySettings {
    /// Proxy server URL (e.g., "http://proxy:8080")
    pub server: String,

    /// Comma-separated domains to bypass proxy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bypass: Option<String>,

    /// Proxy username for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Proxy password for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}
