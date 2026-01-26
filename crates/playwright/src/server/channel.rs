// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Channel - RPC communication proxy for ChannelOwner objects
//
// Architecture Reference:
// - Python: playwright-python/playwright/_impl/_connection.py (Channel class)
// - JavaScript: playwright/.../client/channelOwner.ts (_createChannel method)
// - Java: Channels are implicit in method calls
//
// The Channel provides a typed interface for sending JSON-RPC messages
// to the Playwright server on behalf of a ChannelOwner object.

use crate::error::Result;
use crate::server::connection::ConnectionLike;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;

/// Channel provides RPC communication for a ChannelOwner.
///
/// Every ChannelOwner has a Channel that sends method calls to the
/// Playwright server and receives responses.
///
/// # Architecture
///
/// In the JavaScript implementation, Channel is a Proxy object that
/// intercepts method calls and forwards them to the connection.
///
/// In Python, Channel is a class with explicit method forwarding.
///
/// In Rust, we provide an explicit `send` method that handles:
/// - Serialization of parameters
/// - Sending to connection with object's GUID
/// - Waiting for response
/// - Deserialization of result
///
/// # Example
///
/// ```ignore
/// // Example of using Channel to send RPC calls
/// use playwright_rs::server::channel::Channel;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize)]
/// struct LaunchParams {
///     headless: bool,
/// }
///
/// #[derive(Deserialize)]
/// struct LaunchResult {
///     browser: BrowserRef,
/// }
///
/// // Protocol response references use Arc<str> for performance
/// #[derive(Deserialize)]
/// struct BrowserRef {
///     guid: String, // Simplified for example; actual implementation uses Arc<str>
/// }
///
/// async fn example(channel: &Channel) -> Result<(), Box<dyn std::error::Error>> {
///     let params = LaunchParams { headless: true };
///     let result: LaunchResult = channel.send("launch", params).await?;
///     println!("Browser GUID: {}", result.browser.guid);
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Channel {
    guid: Arc<str>,
    connection: Arc<dyn ConnectionLike>,
}

impl Channel {
    /// Creates a new Channel for the given object GUID.
    ///
    /// # Arguments
    /// * `guid` - The GUID of the ChannelOwner this channel represents
    /// * `connection` - The connection to send messages through
    pub fn new(guid: Arc<str>, connection: Arc<dyn ConnectionLike>) -> Self {
        Self { guid, connection }
    }

    /// Sends a method call to the Playwright server and awaits the response.
    ///
    /// This method:
    /// 1. Serializes `params` to JSON
    /// 2. Sends a JSON-RPC request to the server via the connection
    /// 3. Waits for the response (correlated by request ID)
    /// 4. Deserializes the response to type `R`
    /// 5. Returns the result or an error
    ///
    /// # Type Parameters
    /// * `P` - Parameter type (must be serializable)
    /// * `R` - Result type (must be deserializable)
    ///
    /// # Arguments
    /// * `method` - The method name to call (e.g., "launch", "goto")
    /// * `params` - The parameters to send
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use playwright_rs::server::channel::Channel;
    /// # use serde::{Serialize, Deserialize};
    /// # async fn example(channel: &Channel) -> Result<(), Box<dyn std::error::Error>> {
    /// #[derive(Serialize)]
    /// struct GotoParams<'a> {
    ///     url: &'a str,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct GotoResult {}
    ///
    /// let params = GotoParams { url: "https://example.com" };
    /// let _result: GotoResult = channel.send("goto", params).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send<P: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: P,
    ) -> Result<R> {
        // Serialize params to JSON
        let params_value = serde_json::to_value(params)?;

        // Send message via connection
        let response = self
            .connection
            .send_message(&self.guid, method, params_value)
            .await?;

        // Deserialize response
        serde_json::from_value(response).map_err(Into::into)
    }

    /// Sends a method call with no parameters.
    ///
    /// Convenience method for calls that don't need parameters.
    pub async fn send_no_params<R: DeserializeOwned>(&self, method: &str) -> Result<R> {
        self.send(method, Value::Null).await
    }

    /// Sends a method call that returns no result (void).
    ///
    /// Convenience method for fire-and-forget calls.
    pub async fn send_no_result<P: Serialize>(&self, method: &str, params: P) -> Result<()> {
        let _: Value = self.send(method, params).await?;
        Ok(())
    }

    /// Returns the GUID this channel represents.
    pub fn guid(&self) -> &str {
        &self.guid
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_channel_creation() {
        // Channel creation will be tested in integration tests
        // with a real Connection
    }

    #[test]
    fn test_channel_send() {
        // Channel send will be tested in integration tests
        // with a real server connection
    }
}
