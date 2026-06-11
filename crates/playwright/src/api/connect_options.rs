use std::collections::HashMap;

/// Options for `BrowserType::connect_over_cdp`.
///
/// Only supported for Chromium. Allows connecting to a Chrome DevTools Protocol endpoint,
/// such as those provided by browserless, Chrome with `--remote-debugging-port`, or other
/// CDP-compatible services.
///
/// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-connect-over-cdp>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ConnectOverCdpOptions {
    /// Additional HTTP headers to be sent with the connection request.
    pub headers: Option<HashMap<String, String>>,
    /// Slows down Playwright operations by the specified amount of milliseconds.
    pub slow_mo: Option<f64>,
    /// Maximum time in milliseconds to wait for the connection to be established.
    /// Defaults to 30000 (30 seconds). Pass 0 to disable timeout.
    pub timeout: Option<f64>,
    /// Disables Playwright's default overrides (download behavior, focus/media
    /// emulation) when attaching to an already-running browser — for "attach
    /// without disturbing state" workflows.
    pub no_defaults: Option<bool>,
}

impl ConnectOverCdpOptions {
    /// Creates a new `ConnectOverCdpOptions` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Disable Playwright's default overrides when attaching to a running browser.
    pub fn no_defaults(mut self, no_defaults: bool) -> Self {
        self.no_defaults = Some(no_defaults);
        self
    }

    /// Set additional HTTP headers to send with the connection request.
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

/// Options for `BrowserType::connect`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
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
