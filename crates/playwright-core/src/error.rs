// Error types for playwright-core

use thiserror::Error;

/// Result type alias for playwright-core operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when using playwright-core
#[derive(Debug, Error)]
pub enum Error {
    /// Playwright server binary was not found
    #[error("Playwright server not found at expected location")]
    ServerNotFound,

    /// Failed to launch the Playwright server process
    #[error("Failed to launch Playwright server: {0}")]
    LaunchFailed(String),

    /// Server error (runtime issue with Playwright server)
    #[error("Server error: {0}")]
    ServerError(String),

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
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Target was closed (browser, context, or page)
    #[error("Target closed: {0}")]
    TargetClosed(String),

    /// Unknown protocol object type
    #[error("Unknown protocol object type: {0}")]
    UnknownObjectType(String),

    /// Channel closed unexpectedly
    #[error("Channel closed unexpectedly")]
    ChannelClosed,
}
