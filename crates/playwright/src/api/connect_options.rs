use std::collections::HashMap;

/// Options for `BrowserType::connect`.
#[derive(Debug, Clone, Default)]
pub struct ConnectOptions {
    /// Additional HTTP headers to send with the WebSocket handshake.
    pub headers: Option<HashMap<String, String>>,
    /// Slows down Playwright operations by the specified amount of milliseconds.
    /// Useful so that you can see what is going on.
    pub slow_mo: Option<f64>,
    /// Maximum time in milliseconds to wait for the connection to be established.
    /// Defaults to 30000 (30 seconds). Pass 0 to disable timeout.
    pub timeout: Option<f64>,
}

impl ConnectOptions {
    /// Creates a new `ConnectOptions` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set additional HTTP headers to send with the WebSocket handshake.
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Set slow mo delay in milliseconds.
    pub fn slow_mo(mut self, slow_mo: f64) -> Self {
        self.slow_mo = Some(slow_mo);
        self
    }

    /// Set connection timeout in milliseconds.
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Options for `BrowserType::connect_over_cdp`.
///
/// Used to configure CDP connection parameters when connecting to an existing
/// browser instance via Chrome DevTools Protocol.
#[derive(Debug, Clone, Default)]
pub struct ConnectOverCdpOptions {
    /// Additional HTTP headers to send with the connection.
    /// Used for authentication with secured CDP endpoints.
    pub headers: Option<HashMap<String, String>>,
    /// Slows down Playwright operations by the specified amount of milliseconds.
    /// Useful so that you can see what is going on.
    pub slow_mo: Option<f64>,
    /// Maximum time in milliseconds to wait for the connection to be established.
    /// Defaults to 30000 (30 seconds). Pass 0 to disable timeout.
    pub timeout: Option<f64>,
}

impl ConnectOverCdpOptions {
    /// Creates a new `ConnectOverCdpOptions` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set additional HTTP headers for the connection.
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Set slow mo delay in milliseconds.
    pub fn slow_mo(mut self, slow_mo: f64) -> Self {
        self.slow_mo = Some(slow_mo);
        self
    }

    /// Set connection timeout in milliseconds.
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }
}
