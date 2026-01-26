// Playwright transport layer
//
// Handles bidirectional communication with Playwright server.
// - PipeTransport: stdio pipes with length-prefixed JSON (for launched browsers)
// - WebSocketTransport: WebSocket connection (for connect() to remote browsers)

use crate::Result;
use serde_json::Value as JsonValue;
use std::future::Future;
use std::pin::Pin;

pub mod pipe;
pub mod websocket;

pub use pipe::{PipeTransport, PipeTransportReceiver, send_message};
pub use websocket::WebSocketTransport;

/// Transport trait for abstracting communication mechanisms
pub trait Transport: Send {
    /// Send a JSON message to the server
    fn send(&mut self, message: JsonValue) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// Trait for the sending half of a transport
pub trait TransportSender: Send + Unpin {
    fn send(&mut self, message: JsonValue)
    -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

/// Trait for the receiving half of a transport
pub trait TransportReceiver: Send + Unpin {
    /// Run the receive loop
    fn run(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}
