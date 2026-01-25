// Error types for playwright-core

use thiserror::Error;

/// Result type alias for playwright-core operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when using playwright-core
#[derive(Debug, Error)]
pub enum Error {
    /// Playwright server binary was not found
    ///
    /// The Playwright Node.js driver could not be located.
    /// To resolve this, install Playwright using: `npm install playwright`
    /// Or ensure the PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD environment variable is not set.
    #[error("Playwright server not found. Install with: npm install playwright")]
    ServerNotFound,

    /// Failed to launch the Playwright server process
    ///
    /// The Playwright server process could not be started.
    /// Common causes: Node.js not installed, insufficient permissions, or port already in use.
    /// Details: {0}
    #[error("Failed to launch Playwright server: {0}. Check that Node.js is installed.")]
    LaunchFailed(String),

    /// Server error (runtime issue with Playwright server)
    #[error("Server error: {0}")]
    ServerError(String),

    /// Browser is not installed
    ///
    /// The specified browser has not been installed using Playwright's installation command.
    /// To resolve this, install browsers using the versioned install command to ensure compatibility.
    #[error(
        "Browser '{browser_name}' is not installed.\n\n\
        {message}\n\n\
        To install {browser_name}, run:\n  \
        npx playwright@{playwright_version} install {browser_name}\n\n\
        Or install all browsers:\n  \
        npx playwright@{playwright_version} install\n\n\
        See: https://playwright.dev/docs/browsers"
    )]
    BrowserNotInstalled {
        browser_name: String,
        message: String,
        playwright_version: String,
    },

    /// Failed to establish connection with the server
    #[error("Failed to connect to Playwright server: {0}")]
    ConnectionFailed(String),

    /// Transport-level error (stdio communication)
    #[error("Transport error: {0}")]
    TransportError(String),

    /// Protocol-level error (JSON-RPC)
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Timeout waiting for operation
    ///
    /// Contains context about what operation timed out and the timeout duration.
    /// Common causes include slow network, server not responding, or element not becoming actionable.
    /// Consider increasing the timeout or checking if the target is accessible.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Navigation timeout
    ///
    /// Occurs when page navigation exceeds the specified timeout.
    /// Includes the URL being navigated to and timeout duration.
    #[error("Navigation timeout after {duration_ms}ms navigating to '{url}'")]
    NavigationTimeout { url: String, duration_ms: u64 },

    /// Target was closed (browser, context, or page)
    ///
    /// Occurs when attempting to perform an operation on a closed target.
    /// The target must be recreated before it can be used again.
    #[error("Target closed: Cannot perform operation on closed {target_type}. {context}")]
    TargetClosed {
        target_type: String,
        context: String,
    },

    /// Unknown protocol object type
    #[error("Unknown protocol object type: {0}")]
    UnknownObjectType(String),

    /// Channel closed unexpectedly
    #[error("Channel closed unexpectedly")]
    ChannelClosed,

    /// Invalid argument provided to method
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Element not found by selector
    ///
    /// Includes the selector that was used to locate the element.
    /// This error typically occurs when waiting for an element times out.
    #[error("Element not found: selector '{0}'")]
    ElementNotFound(String),

    /// Assertion timeout (expect API)
    #[error("Assertion timeout: {0}")]
    AssertionTimeout(String),
    /// Object not found in registry (may have been closed/disposed)
    #[error("Object not found (may have been closed): {0}")]
    ObjectNotFound(String),

    /// Invalid path provided
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Error with additional context
    #[error("{0}: {1}")]
    Context(String, #[source] Box<Error>),
}

impl Error {
    /// Adds context to the error
    pub fn context(self, msg: impl Into<String>) -> Self {
        Error::Context(msg.into(), Box::new(self))
    }
}
