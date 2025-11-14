//! JSON-RPC Connection layer for Playwright protocol
//!
//! This module implements the request/response correlation layer on top of the transport.
//! It handles:
//! - Generating unique request IDs
//! - Correlating responses with pending requests
//! - Distinguishing events from responses
//! - Dispatching events to protocol objects
//!
//! # Message Flow
//!
//! 1. Client calls `send_message()` with GUID, method, and params
//! 2. Connection generates unique ID and creates oneshot channel
//! 3. Request is serialized and sent via transport
//! 4. Client awaits on the oneshot receiver
//! 5. Message loop receives response from transport
//! 6. Response is correlated by ID and sent via oneshot channel
//! 7. Client receives result
//!
//! # Example
//!
//! ```ignore
//! # use playwright_core::connection::Connection;
//! # use playwright_core::transport::PipeTransport;
//! # use serde_json::json;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create transport (after launching server)
//! // let (transport, message_rx) = PipeTransport::new(stdin, stdout);
//!
//! // Create connection
//! // let connection = Connection::new(transport, message_rx);
//!
//! // Spawn message loop in background
//! // let conn = connection.clone();
//! // tokio::spawn(async move {
//! //     conn.run().await;
//! // });
//!
//! // Send request and await response
//! // let result = connection.send_message(
//! //     "page@abc123",
//! //     "goto",
//! //     json!({"url": "https://example.com"})
//! // ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # References
//!
//! Based on research of official Playwright bindings:
//! - Python: `playwright/_impl/_connection.py`
//! - Java: `com/microsoft/playwright/impl/Connection.java`
//! - .NET: `Microsoft.Playwright/Core/Connection.cs`

use crate::error::{Error, Result};
use crate::transport::PipeTransport;
use parking_lot::Mutex as ParkingLotMutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::{mpsc, oneshot};

use std::future::Future;
use std::pin::Pin;

/// Trait defining the interface that ChannelOwner needs from a Connection
///
/// This trait allows ChannelOwner to work with Connection without needing to know
/// the generic parameters W and R. The Connection struct implements this trait.
pub trait ConnectionLike: Send + Sync {
    /// Send a message to the Playwright server and await response
    fn send_message(
        &self,
        guid: &str,
        method: &str,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>>;

    /// Register an object in the connection's registry
    fn register_object(
        &self,
        guid: Arc<str>,
        object: Arc<dyn ChannelOwner>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

    /// Unregister an object from the connection's registry
    fn unregister_object(&self, guid: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

    /// Get an object by GUID
    fn get_object(&self, guid: &str) -> AsyncChannelOwnerResult<'_>;
}

// Type alias for complex async return type
type AsyncChannelOwnerResult<'a> =
    Pin<Box<dyn Future<Output = Result<Arc<dyn ChannelOwner>>> + Send + 'a>>;

// Forward declaration - will be used for object registry
use crate::channel_owner::ChannelOwner;

/// Metadata attached to every Playwright protocol message
///
/// Contains timing information and optional location data for debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Unix timestamp in milliseconds
    #[serde(rename = "wallTime")]
    pub wall_time: i64,
    /// Whether this is an internal call (not user-facing API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<bool>,
    /// Source location where the API was called
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    /// Optional title for the operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Source code location for a protocol call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// Source file path
    pub file: String,
    /// Line number (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i32>,
    /// Column number (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i32>,
}

impl Metadata {
    /// Create minimal metadata with current timestamp
    pub fn now() -> Self {
        Self {
            wall_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            internal: Some(false),
            location: None,
            title: None,
        }
    }
}

/// Protocol request message sent to Playwright server
///
/// Format matches Playwright's JSON-RPC protocol:
/// ```json
/// {
///   "id": 42,
///   "guid": "page@3ee5e10621a15eaf80cb985dbccb9a28",
///   "method": "goto",
///   "params": {
///     "url": "https://example.com"
///   },
///   "metadata": {
///     "wallTime": 1699876543210,
///     "internal": false
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// Unique request ID for correlating responses
    pub id: u32,
    /// GUID of the target object (format: "type@hash")
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub guid: Arc<str>,
    /// Method name to invoke
    pub method: String,
    /// Method parameters as JSON object
    pub params: Value,
    /// Metadata with timing and location information
    pub metadata: Metadata,
}

/// Serde helpers for `Arc<str>` serialization
///
/// These helpers allow `Arc<str>` to be serialized/deserialized as a regular string in JSON.
/// This is used for GUID fields throughout the protocol layer for performance optimization.
pub fn serialize_arc_str<S>(arc: &Arc<str>, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(arc)
}

pub fn deserialize_arc_str<'de, D>(deserializer: D) -> std::result::Result<Arc<str>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    Ok(Arc::from(s.as_str()))
}

/// Protocol response message from Playwright server
///
/// Format matches Playwright's JSON-RPC protocol:
/// ```json
/// {
///   "id": 42,
///   "result": { "response": { "guid": "response@..." } }
/// }
/// ```
///
/// Or with error:
/// ```json
/// {
///   "id": 42,
///   "error": {
///     "error": {
///       "message": "Navigation timeout",
///       "name": "TimeoutError",
///       "stack": "..."
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Request ID this response correlates to
    pub id: u32,
    /// Success result (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error result (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorWrapper>,
}

/// Wrapper for protocol error payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorWrapper {
    pub error: ErrorPayload,
}

/// Protocol error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    /// Error message
    pub message: String,
    /// Error type name (e.g., "TimeoutError", "TargetClosedError")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Stack trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

/// Protocol event message from Playwright server
///
/// Events are distinguished from responses by the absence of an `id` field:
/// ```json
/// {
///   "guid": "page@3ee5e10621a15eaf80cb985dbccb9a28",
///   "method": "console",
///   "params": {
///     "message": { "type": "log", "text": "Hello world" }
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// GUID of the object that emitted the event
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub guid: Arc<str>,
    /// Event method name
    pub method: String,
    /// Event parameters as JSON object
    pub params: Value,
}

/// Discriminated union of protocol messages
///
/// Uses serde's `untagged` to distinguish based on presence of `id` field:
/// - Messages with `id` are responses
/// - Messages without `id` are events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    /// Response message (has `id` field)
    Response(Response),
    /// Event message (no `id` field)
    Event(Event),
}

/// Type alias for the object registry mapping GUIDs to ChannelOwner objects
type ObjectRegistry = HashMap<Arc<str>, Arc<dyn ChannelOwner>>;

/// JSON-RPC connection to Playwright server
///
/// Manages request/response correlation and event dispatch.
/// Uses sequential request IDs and oneshot channels for correlation.
///
/// # Thread Safety
///
/// Connection is thread-safe and can be shared across async tasks using `Arc`.
/// Multiple concurrent requests are supported.
///
/// # Architecture
///
/// This follows the pattern from official Playwright bindings:
/// - Python: Direct callback on message receive
/// - Java: Callback map with synchronized access
/// - .NET: ConcurrentDictionary with TaskCompletionSource
///
/// Rust implementation uses:
/// - `AtomicU32` for thread-safe ID generation
/// - `Arc<Mutex<HashMap>>` for callback storage
/// - `tokio::sync::oneshot` for request/response correlation
pub struct Connection<W, R>
where
    W: tokio::io::AsyncWrite + Unpin + Send + Sync + 'static,
    R: tokio::io::AsyncRead + Unpin + Send + Sync + 'static,
{
    /// Sequential request ID counter (atomic for thread safety)
    last_id: AtomicU32,
    /// Pending request callbacks keyed by request ID
    callbacks: Arc<TokioMutex<HashMap<u32, oneshot::Sender<Result<Value>>>>>,
    /// Stdin for sending messages (mutex-wrapped for concurrent sends)
    stdin: Arc<TokioMutex<W>>,
    /// Receiver for incoming messages from transport
    message_rx: Arc<TokioMutex<Option<mpsc::UnboundedReceiver<Value>>>>,
    /// Receiver half of transport (owned by run loop, only needed once)
    transport_receiver: Arc<TokioMutex<Option<crate::transport::PipeTransportReceiver<R>>>>,
    /// Registry of all protocol objects by GUID (parking_lot for sync+async access)
    objects: Arc<ParkingLotMutex<ObjectRegistry>>,
}

// Type alias for Connection using concrete transport (most common case)
pub type RealConnection = Connection<tokio::process::ChildStdin, tokio::process::ChildStdout>;

impl<W, R> Connection<W, R>
where
    W: tokio::io::AsyncWrite + Unpin + Send + Sync + 'static,
    R: tokio::io::AsyncRead + Unpin + Send + Sync + 'static,
{
    /// Create a new Connection with the given transport
    ///
    /// # Arguments
    ///
    /// * `transport` - Transport connected to Playwright server
    /// * `message_rx` - Receiver for incoming messages from transport
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use playwright_core::connection::Connection;
    /// # use playwright_core::transport::PipeTransport;
    /// # use tokio::io::duplex;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let (stdin_read, stdin_write) = duplex(1024);
    /// let (stdout_read, stdout_write) = duplex(1024);
    ///
    /// let (transport, message_rx) = PipeTransport::new(stdin_write, stdout_read);
    /// let connection = Connection::new(transport, message_rx);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(transport: PipeTransport<W, R>, message_rx: mpsc::UnboundedReceiver<Value>) -> Self {
        // Split transport into send and receive parts
        // This prevents deadlock: stdin can be locked for sends while
        // the transport receiver runs independently
        let (stdin, transport_receiver) = transport.into_parts();

        Self {
            last_id: AtomicU32::new(0),
            callbacks: Arc::new(TokioMutex::new(HashMap::new())),
            stdin: Arc::new(TokioMutex::new(stdin)),
            message_rx: Arc::new(TokioMutex::new(Some(message_rx))),
            transport_receiver: Arc::new(TokioMutex::new(Some(transport_receiver))),
            objects: Arc::new(ParkingLotMutex::new(HashMap::new())),
        }
    }

    /// Send a message to the Playwright server and await response
    ///
    /// This method:
    /// 1. Generates a unique request ID
    /// 2. Creates a oneshot channel for the response
    /// 3. Stores the channel sender in the callbacks map
    /// 4. Serializes and sends the request via transport
    /// 5. Awaits the response on the receiver
    ///
    /// # Arguments
    ///
    /// * `guid` - GUID of the target object (e.g., "page@abc123")
    /// * `method` - Method name to invoke (e.g., "goto")
    /// * `params` - Method parameters as JSON value
    ///
    /// # Returns
    ///
    /// The result value from the server, or an error if:
    /// - Transport send fails
    /// - Server returns an error
    /// - Connection is closed before response arrives
    ///
    /// See module-level documentation for usage examples.
    pub async fn send_message(&self, guid: &str, method: &str, params: Value) -> Result<Value> {
        // Generate unique ID (atomic increment for thread safety)
        let id = self.last_id.fetch_add(1, Ordering::SeqCst);

        tracing::debug!(
            "Sending message: id={}, guid='{}', method='{}'",
            id,
            guid,
            method
        );

        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();

        // Store callback
        self.callbacks.lock().await.insert(id, tx);

        // Build request with metadata
        let request = Request {
            id,
            guid: Arc::from(guid),
            method: method.to_string(),
            params,
            metadata: Metadata::now(),
        };

        // Send via stdin using the helper function
        let request_value = serde_json::to_value(&request)?;
        tracing::debug!("Request JSON: {}", request_value);

        match crate::transport::send_message(&mut *self.stdin.lock().await, request_value).await {
            Ok(()) => tracing::debug!("Message sent successfully, awaiting response"),
            Err(e) => {
                tracing::error!("Failed to send message: {:?}", e);
                return Err(e);
            }
        }

        // Await response
        tracing::debug!("Waiting for response to ID {}", id);
        rx.await
            .map_err(|_| Error::ChannelClosed)
            .and_then(|result| result)
    }

    /// Initialize the Playwright connection and return the root Playwright object
    ///
    /// This method implements the initialization handshake with the Playwright server:
    /// 1. Creates a temporary Root object
    /// 2. Sends "initialize" message with sdkLanguage="rust"
    /// 3. Server creates BrowserType objects (sends `__create__` messages)
    /// 4. Server responds with Playwright GUID
    /// 5. Looks up Playwright object from registry (guaranteed to exist)
    ///
    /// The `initialize` message is synchronous - by the time the response arrives,
    /// all protocol objects have been created and registered.
    ///
    /// # Returns
    ///
    /// An `Arc<dyn ChannelOwner>` that is the Playwright object. Callers should downcast
    /// to `Playwright` type.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Initialize message fails to send
    /// - Server returns protocol error
    /// - Response is missing Playwright GUID
    /// - Playwright object not found in registry
    /// - Timeout after 30 seconds
    ///
    /// See module-level documentation for usage examples.
    ///
    /// See also:
    /// - [ADR 0002: Initialization Flow](../../docs/adr/0002-initialization-flow.md)
    /// - Python: <https://github.com/microsoft/playwright-python/blob/main/playwright/_impl/_connection.py>
    pub async fn initialize_playwright(self: &Arc<Self>) -> Result<Arc<dyn ChannelOwner>> {
        use crate::protocol::Root;
        use std::time::Duration;

        // Create temporary Root object for initialization
        // Root has empty GUID ("") and acts as parent for top-level objects
        let root = Arc::new(Root::new(Arc::clone(self) as Arc<dyn ConnectionLike>))
            as Arc<dyn ChannelOwner>;

        // CRITICAL: Register Root in objects map with empty GUID
        // This allows __create__ messages to find Root as their parent
        // Matches Python's behavior where RootChannelOwner auto-registers
        self.objects.lock().insert(Arc::from(""), root.clone());

        tracing::debug!("Root object registered, sending initialize message");

        // Downcast to Root type to call initialize()
        let root_typed = root
            .as_any()
            .downcast_ref::<Root>()
            .expect("Root object should be Root type");

        // Send initialize message (blocks until server responds)
        // Add timeout to prevent hanging forever
        let response = tokio::time::timeout(Duration::from_secs(30), root_typed.initialize())
            .await
            .map_err(|_| {
                Error::Timeout("Playwright initialization timeout after 30 seconds".to_string())
            })??;

        // Extract Playwright GUID from response
        // Response format: { "playwright": { "guid": "playwright" } }
        let playwright_guid = response["playwright"]["guid"].as_str().ok_or_else(|| {
            Error::ProtocolError("Initialize response missing 'playwright.guid' field".to_string())
        })?;

        tracing::debug!("Initialized Playwright with GUID: {}", playwright_guid);

        // Get Playwright object from registry
        // By this point, the server has sent all __create__ messages
        // and the Playwright object is already registered
        let playwright_obj = self.get_object(playwright_guid).await?;

        // Verify it's actually a Playwright object
        playwright_obj
            .as_any()
            .downcast_ref::<crate::protocol::Playwright>()
            .ok_or_else(|| {
                Error::ProtocolError(format!(
                    "Object with GUID '{}' is not a Playwright instance",
                    playwright_guid
                ))
            })?;

        // Cleanup: Unregister Root after initialization
        // Root is only needed during the initialization handshake
        let empty_guid: Arc<str> = Arc::from("");
        self.objects.lock().remove(&empty_guid);
        tracing::debug!("Root object unregistered after successful initialization");

        // Return the Arc<dyn ChannelOwner>
        // The high-level API will handle downcasting
        Ok(playwright_obj)
    }

    /// Run the message dispatch loop
    ///
    /// This method continuously reads messages from the transport and dispatches them:
    /// - Responses (with `id`) are correlated with pending requests
    /// - Events (without `id`) are dispatched to protocol objects (TODO: Future phase - event handling)
    ///
    /// The loop runs until the transport channel is closed.
    ///
    /// # Usage
    ///
    /// This method should be spawned in a background task:
    ///
    /// ```ignore
    /// # use playwright_core::connection::Connection;
    /// # use playwright_core::transport::PipeTransport;
    /// # use std::sync::Arc;
    /// # use tokio::io::duplex;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let (stdin_read, stdin_write) = duplex(1024);
    /// # let (stdout_read, stdout_write) = duplex(1024);
    /// # let (transport, message_rx) = PipeTransport::new(stdin_write, stdout_read);
    /// # let connection = Arc::new(Connection::new(transport, message_rx));
    /// let conn = Arc::clone(&connection);
    /// tokio::spawn(async move {
    ///     conn.run().await;
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(self: &Arc<Self>) {
        // Take the transport receiver (can only be called once)
        let transport_receiver = self
            .transport_receiver
            .lock()
            .await
            .take()
            .expect("run() can only be called once - transport receiver already taken");

        // Spawn transport read loop WITHOUT any locks
        // This prevents deadlock: the receiver owns stdout and runs independently
        let transport_handle = tokio::spawn(async move {
            if let Err(e) = transport_receiver.run().await {
                tracing::error!("Transport error: {}", e);
            }
        });

        // Take the message receiver out of the Option (can only be called once)
        let mut message_rx = self
            .message_rx
            .lock()
            .await
            .take()
            .expect("run() can only be called once - message receiver already taken");

        while let Some(message_value) = message_rx.recv().await {
            // Parse message as Response or Event
            match serde_json::from_value::<Message>(message_value) {
                Ok(message) => {
                    if let Err(e) = self.dispatch_internal(message).await {
                        tracing::error!("Error dispatching message: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse message: {}", e);
                }
            }
        }

        tracing::debug!("Message loop ended (transport closed)");

        // Wait for transport task to finish
        let _ = transport_handle.await;
    }

    /// Dispatch an incoming message from the transport
    ///
    /// This method:
    /// - Parses the message as Response or Event
    /// - For responses: correlates by ID and completes the oneshot channel
    /// - For events: dispatches to the appropriate object (TODO: Future phase - event handling)
    ///
    /// # Arguments
    ///
    /// * `message` - Parsed protocol message
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Response ID doesn't match any pending request
    /// - Event GUID doesn't match any registered object
    #[cfg(test)]
    pub async fn dispatch(self: &Arc<Self>, message: Message) -> Result<()> {
        self.dispatch_internal(message).await
    }

    async fn dispatch_internal(self: &Arc<Self>, message: Message) -> Result<()> {
        tracing::debug!("Dispatching message: {:?}", message);
        match message {
            Message::Response(response) => {
                tracing::debug!("Processing response for ID: {}", response.id);
                // Correlate response with pending request
                let callback = self
                    .callbacks
                    .lock()
                    .await
                    .remove(&response.id)
                    .ok_or_else(|| {
                        Error::ProtocolError(format!(
                            "Cannot find request to respond: id={}",
                            response.id
                        ))
                    })?;

                // Convert protocol error to Rust error
                let result = if let Some(error_wrapper) = response.error {
                    Err(parse_protocol_error(error_wrapper.error))
                } else {
                    Ok(response.result.unwrap_or(Value::Null))
                };

                // Complete the oneshot channel (ignore if receiver was dropped)
                let _ = callback.send(result);
                Ok(())
            }
            Message::Event(event) => {
                // Handle special protocol methods
                match event.method.as_str() {
                    "__create__" => self.handle_create(&event).await,
                    "__dispose__" => self.handle_dispose(&event).await,
                    "__adopt__" => self.handle_adopt(&event).await,
                    _ => {
                        // Regular event - dispatch to object
                        match self.objects.lock().get(&event.guid).cloned() {
                            Some(object) => {
                                object.on_event(&event.method, event.params);
                                Ok(())
                            }
                            None => {
                                tracing::warn!(
                                    "Event for unknown object: guid={}, method={}",
                                    event.guid,
                                    event.method
                                );
                                Ok(())
                            }
                        }
                    }
                }
            }
        }
    }

    /// Handle `__create__` protocol message
    ///
    /// Creates a new protocol object and registers it in the connection.
    async fn handle_create(self: &Arc<Self>, event: &Event) -> Result<()> {
        use crate::channel_owner::ParentOrConnection;
        use crate::object_factory::create_object;

        // Extract parameters from event
        let type_name = event.params["type"]
            .as_str()
            .ok_or_else(|| Error::ProtocolError("__create__ missing 'type'".to_string()))?
            .to_string();

        let object_guid: Arc<str> = Arc::from(
            event.params["guid"]
                .as_str()
                .ok_or_else(|| Error::ProtocolError("__create__ missing 'guid'".to_string()))?,
        );

        eprintln!(
            "DEBUG __create__: type={}, guid={}, parent_guid={}",
            type_name, object_guid, event.guid
        );

        let initializer = event.params["initializer"].clone();

        // Determine parent
        let parent_obj = self
            .objects
            .lock()
            .get(&event.guid)
            .cloned()
            .ok_or_else(|| {
                eprintln!(
                    "DEBUG: Parent object not found for type={}, parent_guid={}",
                    type_name, event.guid
                );
                Error::ProtocolError(format!("Parent object not found: {}", event.guid))
            })?;

        // Create object using factory
        // Special case: Playwright object needs Connection, not Parent
        let parent_or_conn = if type_name == "Playwright" && event.guid.is_empty() {
            ParentOrConnection::Connection(Arc::clone(self) as Arc<dyn ConnectionLike>)
        } else {
            ParentOrConnection::Parent(parent_obj.clone())
        };

        let object = match create_object(
            parent_or_conn,
            type_name.clone(),
            object_guid.clone(),
            initializer,
        )
        .await
        {
            Ok(obj) => obj,
            Err(e) => {
                eprintln!(
                    "DEBUG: Failed to create object type={}, guid={}, error={}",
                    type_name, object_guid, e
                );
                return Err(e);
            }
        };

        // Register in connection
        self.register_object(Arc::clone(&object_guid), object.clone())
            .await;

        // Register in parent
        parent_obj.add_child(Arc::clone(&object_guid), object);

        eprintln!(
            "DEBUG: Successfully created and registered object: type={}, guid={}",
            type_name, object_guid
        );
        tracing::debug!("Created object: type={}, guid={}", type_name, object_guid);

        Ok(())
    }

    /// Handle `__dispose__` protocol message
    ///
    /// Disposes an object and removes it from the registry.
    async fn handle_dispose(&self, event: &Event) -> Result<()> {
        use crate::channel_owner::DisposeReason;

        let reason = match event.params.get("reason").and_then(|r| r.as_str()) {
            Some("gc") => DisposeReason::GarbageCollected,
            _ => DisposeReason::Closed,
        };

        // Get object from registry
        let object = self.objects.lock().get(&event.guid).cloned();

        if let Some(obj) = object {
            // Dispose the object (this will remove from parent and unregister)
            obj.dispose(reason);

            tracing::debug!("Disposed object: guid={}", event.guid);
        } else {
            tracing::warn!("Dispose for unknown object: guid={}", event.guid);
        }

        Ok(())
    }

    /// Handle `__adopt__` protocol message
    ///
    /// Moves a child object from one parent to another.
    async fn handle_adopt(&self, event: &Event) -> Result<()> {
        let child_guid: Arc<str> = Arc::from(
            event.params["guid"]
                .as_str()
                .ok_or_else(|| Error::ProtocolError("__adopt__ missing 'guid'".to_string()))?,
        );

        // Get new parent and child from registry
        let new_parent = self.objects.lock().get(&event.guid).cloned();
        let child = self.objects.lock().get(&child_guid).cloned();

        match (new_parent, child) {
            (Some(parent), Some(child_obj)) => {
                parent.adopt(child_obj);
                tracing::debug!(
                    "Adopted object: child={}, new_parent={}",
                    child_guid,
                    event.guid
                );
                Ok(())
            }
            (None, _) => Err(Error::ProtocolError(format!(
                "Parent object not found: {}",
                event.guid
            ))),
            (_, None) => Err(Error::ProtocolError(format!(
                "Child object not found: {}",
                child_guid
            ))),
        }
    }
}

/// Parse protocol error into Rust error type
fn parse_protocol_error(error: ErrorPayload) -> Error {
    match error.name.as_deref() {
        Some("TimeoutError") => Error::Timeout(error.message),
        Some("TargetClosedError") => Error::TargetClosed {
            target_type: "target".to_string(),
            context: error.message,
        },
        _ => Error::ProtocolError(error.message),
    }
}

// Implement ConnectionLike trait for Connection
impl<W, R> ConnectionLike for Connection<W, R>
where
    W: tokio::io::AsyncWrite + Unpin + Send + Sync + 'static,
    R: tokio::io::AsyncRead + Unpin + Send + Sync + 'static,
{
    fn send_message(
        &self,
        guid: &str,
        method: &str,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>> {
        // Convert to owned strings to avoid lifetime issues
        let guid = guid.to_string();
        let method = method.to_string();

        // Box the future returned by the async method
        Box::pin(async move { Connection::send_message(self, &guid, &method, params).await })
    }

    fn register_object(
        &self,
        guid: Arc<str>,
        object: Arc<dyn ChannelOwner>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            self.objects.lock().insert(guid, object);
        })
    }

    fn unregister_object(&self, guid: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let guid_arc: Arc<str> = Arc::from(guid);
        Box::pin(async move {
            self.objects.lock().remove(&guid_arc);
        })
    }

    fn get_object(&self, guid: &str) -> AsyncChannelOwnerResult<'_> {
        let guid_arc: Arc<str> = Arc::from(guid);
        Box::pin(async move {
            self.objects.lock().get(&guid_arc).cloned().ok_or_else(|| {
                // Determine target type from GUID prefix
                let target_type = if guid_arc.starts_with("page@") {
                    "Page"
                } else if guid_arc.starts_with("frame@") {
                    "Frame"
                } else if guid_arc.starts_with("browser-context@") {
                    "BrowserContext"
                } else if guid_arc.starts_with("browser@") {
                    "Browser"
                } else {
                    return Error::ProtocolError(format!("Object not found: {}", guid_arc));
                };

                Error::TargetClosed {
                    target_type: target_type.to_string(),
                    context: format!("Object not found: {}", guid_arc),
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    // Helper to create test connection with mock transport
    fn create_test_connection() -> (
        Connection<tokio::io::DuplexStream, tokio::io::DuplexStream>,
        tokio::io::DuplexStream,
        tokio::io::DuplexStream,
    ) {
        let (stdin_read, stdin_write) = duplex(1024);
        let (stdout_read, stdout_write) = duplex(1024);

        let (transport, message_rx) = PipeTransport::new(stdin_write, stdout_read);
        let connection = Connection::new(transport, message_rx);

        (connection, stdin_read, stdout_write)
    }

    #[test]
    fn test_request_id_increments() {
        let (connection, _, _) = create_test_connection();

        // Generate IDs by incrementing the counter directly
        let id1 = connection.last_id.fetch_add(1, Ordering::SeqCst);
        let id2 = connection.last_id.fetch_add(1, Ordering::SeqCst);
        let id3 = connection.last_id.fetch_add(1, Ordering::SeqCst);

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);
    }

    #[test]
    fn test_request_format() {
        let request = Request {
            id: 0,
            guid: Arc::from("page@abc123"),
            method: "goto".to_string(),
            params: serde_json::json!({"url": "https://example.com"}),
            metadata: Metadata::now(),
        };

        assert_eq!(request.id, 0);
        assert_eq!(request.guid.as_ref(), "page@abc123");
        assert_eq!(request.method, "goto");
        assert_eq!(request.params["url"], "https://example.com");
    }

    #[tokio::test]
    async fn test_dispatch_response_success() {
        let (connection, _, _) = create_test_connection();

        // Generate ID
        let id = connection.last_id.fetch_add(1, Ordering::SeqCst);

        // Create oneshot channel and store callback
        let (tx, rx) = oneshot::channel();
        connection.callbacks.lock().await.insert(id, tx);

        // Simulate response from server
        let response = Message::Response(Response {
            id,
            result: Some(serde_json::json!({"status": "ok"})),
            error: None,
        });

        // Dispatch response
        Arc::new(connection).dispatch(response).await.unwrap();

        // Verify result
        let result = rx.await.unwrap().unwrap();
        assert_eq!(result["status"], "ok");
    }

    #[tokio::test]
    async fn test_dispatch_response_error() {
        let (connection, _, _) = create_test_connection();

        // Generate ID
        let id = connection.last_id.fetch_add(1, Ordering::SeqCst);

        // Create oneshot channel and store callback
        let (tx, rx) = oneshot::channel();
        connection.callbacks.lock().await.insert(id, tx);

        // Simulate error response from server
        let response = Message::Response(Response {
            id,
            result: None,
            error: Some(ErrorWrapper {
                error: ErrorPayload {
                    message: "Navigation timeout".to_string(),
                    name: Some("TimeoutError".to_string()),
                    stack: None,
                },
            }),
        });

        // Dispatch response
        Arc::new(connection).dispatch(response).await.unwrap();

        // Verify error
        let result = rx.await.unwrap();
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Timeout(msg) => assert_eq!(msg, "Navigation timeout"),
            _ => panic!("Expected Timeout error"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_invalid_id() {
        let (connection, _, _) = create_test_connection();

        // Create response with ID that doesn't match any request
        let response = Message::Response(Response {
            id: 999,
            result: Some(Value::Null),
            error: None,
        });

        // Dispatch should return error
        let result = Arc::new(connection).dispatch(response).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::ProtocolError(msg) => assert!(msg.contains("Cannot find request")),
            _ => panic!("Expected ProtocolError"),
        }
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        let (connection, _, _) = create_test_connection();
        let connection = Arc::new(connection);

        // Create callbacks for multiple requests
        let id1 = connection.last_id.fetch_add(1, Ordering::SeqCst);
        let id2 = connection.last_id.fetch_add(1, Ordering::SeqCst);
        let id3 = connection.last_id.fetch_add(1, Ordering::SeqCst);

        let (tx1, rx1) = oneshot::channel();
        let (tx2, rx2) = oneshot::channel();
        let (tx3, rx3) = oneshot::channel();

        connection.callbacks.lock().await.insert(id1, tx1);
        connection.callbacks.lock().await.insert(id2, tx2);
        connection.callbacks.lock().await.insert(id3, tx3);

        // Verify IDs are unique
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);

        // Simulate responses arriving in different order
        let conn1 = Arc::clone(&connection);
        let conn2 = Arc::clone(&connection);
        let conn3 = Arc::clone(&connection);

        let handle1 = tokio::spawn(async move {
            conn1
                .dispatch(Message::Response(Response {
                    id: 1,
                    result: Some(serde_json::json!({"page": "2"})),
                    error: None,
                }))
                .await
                .unwrap();
        });

        let handle2 = tokio::spawn(async move {
            conn2
                .dispatch(Message::Response(Response {
                    id: 0,
                    result: Some(serde_json::json!({"page": "1"})),
                    error: None,
                }))
                .await
                .unwrap();
        });

        let handle3 = tokio::spawn(async move {
            conn3
                .dispatch(Message::Response(Response {
                    id: 2,
                    result: Some(serde_json::json!({"page": "3"})),
                    error: None,
                }))
                .await
                .unwrap();
        });

        // Wait for all dispatches to complete
        handle1.await.unwrap();
        handle2.await.unwrap();
        handle3.await.unwrap();

        // Verify each receiver gets the correct response
        let result1 = rx1.await.unwrap().unwrap();
        let result2 = rx2.await.unwrap().unwrap();
        let result3 = rx3.await.unwrap().unwrap();

        assert_eq!(result1["page"], "1");
        assert_eq!(result2["page"], "2");
        assert_eq!(result3["page"], "3");
    }

    #[test]
    fn test_message_deserialization_response() {
        let json = r#"{"id": 42, "result": {"status": "ok"}}"#;
        let message: Message = serde_json::from_str(json).unwrap();

        match message {
            Message::Response(response) => {
                assert_eq!(response.id, 42);
                assert!(response.result.is_some());
                assert!(response.error.is_none());
            }
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn test_message_deserialization_event() {
        let json = r#"{"guid": "page@abc", "method": "console", "params": {"text": "hello"}}"#;
        let message: Message = serde_json::from_str(json).unwrap();

        match message {
            Message::Event(event) => {
                assert_eq!(event.guid.as_ref(), "page@abc");
                assert_eq!(event.method, "console");
                assert_eq!(event.params["text"], "hello");
            }
            _ => panic!("Expected Event"),
        }
    }

    #[test]
    fn test_error_type_parsing() {
        // TimeoutError
        let error = parse_protocol_error(ErrorPayload {
            message: "timeout".to_string(),
            name: Some("TimeoutError".to_string()),
            stack: None,
        });
        assert!(matches!(error, Error::Timeout(_)));

        // TargetClosedError
        let error = parse_protocol_error(ErrorPayload {
            message: "closed".to_string(),
            name: Some("TargetClosedError".to_string()),
            stack: None,
        });
        assert!(matches!(error, Error::TargetClosed { .. }));

        // Generic error
        let error = parse_protocol_error(ErrorPayload {
            message: "generic".to_string(),
            name: None,
            stack: None,
        });
        assert!(matches!(error, Error::ProtocolError(_)));
    }
}
