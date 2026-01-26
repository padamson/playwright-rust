//! JSON-RPC Connection layer for Playwright protocol

use crate::error::{Error, Result};
use crate::server::transport::{TransportReceiver, TransportSender};
use parking_lot::Mutex as ParkingLotMutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::{mpsc, oneshot};

use std::future::Future;
use std::pin::Pin;

/// Trait defining the interface that ChannelOwner needs from a Connection
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

type AsyncChannelOwnerResult<'a> =
    Pin<Box<dyn Future<Output = Result<Arc<dyn ChannelOwner>>> + Send + 'a>>;

use crate::server::channel_owner::ChannelOwner;

/// Metadata attached to every Playwright protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(rename = "wallTime")]
    pub wall_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i32>,
}

impl Metadata {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: u32,
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub guid: Arc<str>,
    pub method: String,
    #[serde(skip_serializing_if = "is_value_null")]
    pub params: Value,
    pub metadata: Metadata,
}

fn is_value_null(v: &Value) -> bool {
    v.is_null()
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorWrapper>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorWrapper {
    pub error: ErrorPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub guid: Arc<str>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    Response(Response),
    Event(Event),
}

type ObjectRegistry = HashMap<Arc<str>, Arc<dyn ChannelOwner>>;

/// JSON-RPC connection to Playwright server
pub struct Connection {
    last_id: AtomicU32,
    callbacks: Arc<TokioMutex<HashMap<u32, oneshot::Sender<Result<Value>>>>>,
    sender: Arc<TokioMutex<Box<dyn TransportSender>>>,
    message_rx: Arc<TokioMutex<Option<mpsc::UnboundedReceiver<Value>>>>,
    transport_receiver: Arc<TokioMutex<Option<Box<dyn TransportReceiver>>>>,
    objects: Arc<ParkingLotMutex<ObjectRegistry>>,
}

// Type alias for compatibility (though generic parameters are gone, we can keep alias if needed)
pub type RealConnection = Connection;

impl Connection {
    pub fn new(
        sender: impl TransportSender + 'static,
        receiver: impl TransportReceiver + 'static,
        message_rx: mpsc::UnboundedReceiver<Value>,
    ) -> Self {
        Self {
            last_id: AtomicU32::new(0),
            callbacks: Arc::new(TokioMutex::new(HashMap::new())),
            sender: Arc::new(TokioMutex::new(Box::new(sender))),
            message_rx: Arc::new(TokioMutex::new(Some(message_rx))),
            transport_receiver: Arc::new(TokioMutex::new(Some(Box::new(receiver)))),
            objects: Arc::new(ParkingLotMutex::new(HashMap::new())),
        }
    }

    pub async fn send_message(&self, guid: String, method: String, params: Value) -> Result<Value> {
        let id = self.last_id.fetch_add(1, Ordering::SeqCst);

        tracing::debug!(
            "Sending message: id={}, guid='{}', method='{}'",
            id,
            guid,
            method
        );

        let (tx, rx) = oneshot::channel();
        self.callbacks.lock().await.insert(id, tx);

        let request = Request {
            id,
            guid: Arc::from(guid),
            method,
            params,
            metadata: Metadata::now(),
        };

        let request_value = serde_json::to_value(&request)?;
        tracing::debug!("Request JSON: {}", request_value);

        match self.sender.lock().await.send(request_value).await {
            Ok(()) => tracing::debug!("Message sent successfully, awaiting response"),
            Err(e) => {
                tracing::error!("Failed to send message: {:?}", e);
                return Err(e);
            }
        }

        tracing::debug!("Waiting for response to ID {}", id);
        rx.await
            .map_err(|_| Error::ChannelClosed)
            .and_then(|result| result)
    }

    pub async fn initialize_playwright(self: &Arc<Self>) -> Result<Arc<dyn ChannelOwner>> {
        use crate::protocol::Root;
        use std::time::Duration;

        let root = Arc::new(Root::new(Arc::clone(self) as Arc<dyn ConnectionLike>))
            as Arc<dyn ChannelOwner>;

        self.objects.lock().insert(Arc::from(""), root.clone());

        tracing::debug!("Root object registered, sending initialize message");

        let root_typed = root
            .as_any()
            .downcast_ref::<Root>()
            .expect("Root object should be Root type");

        let response = tokio::time::timeout(Duration::from_secs(30), root_typed.initialize())
            .await
            .map_err(|_| {
                Error::Timeout("Playwright initialization timeout after 30 seconds".to_string())
            })??;

        let playwright_guid = response["playwright"]["guid"].as_str().ok_or_else(|| {
            Error::ProtocolError("Initialize response missing 'playwright.guid' field".to_string())
        })?;

        tracing::debug!("Initialized Playwright with GUID: {}", playwright_guid);

        let playwright_obj = self.wait_for_object(playwright_guid).await?;

        playwright_obj
            .as_any()
            .downcast_ref::<crate::protocol::Playwright>()
            .ok_or_else(|| {
                Error::ProtocolError(format!(
                    "Object with GUID '{}' is not a Playwright instance",
                    playwright_guid
                ))
            })?;

        let empty_guid: Arc<str> = Arc::from("");
        self.objects.lock().remove(&empty_guid);
        tracing::debug!("Root object unregistered after successful initialization");

        Ok(playwright_obj)
    }

    pub async fn run(self: &Arc<Self>) {
        let mut transport_receiver = self
            .transport_receiver
            .lock()
            .await
            .take()
            .expect("run() can only be called once - transport receiver already taken");

        let transport_handle = tokio::spawn(async move {
            if let Err(e) = transport_receiver.run().await {
                tracing::error!("Transport error: {}", e);
            }
        });

        let mut message_rx = self
            .message_rx
            .lock()
            .await
            .take()
            .expect("run() can only be called once - message receiver already taken");

        while let Some(message_value) = message_rx.recv().await {
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
        let _ = transport_handle.await;
    }

    #[cfg(test)]
    pub async fn dispatch(self: &Arc<Self>, message: Message) -> Result<()> {
        self.dispatch_internal(message).await
    }

    async fn dispatch_internal(self: &Arc<Self>, message: Message) -> Result<()> {
        tracing::debug!("Dispatching message: {:?}", message);
        match message {
            Message::Response(response) => {
                tracing::debug!("Processing response for ID: {}", response.id);
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

                let result = if let Some(error_wrapper) = response.error {
                    Err(parse_protocol_error(error_wrapper.error))
                } else {
                    Ok(response.result.unwrap_or(Value::Null))
                };

                let _ = callback.send(result);
                Ok(())
            }
            Message::Event(event) => match event.method.as_str() {
                "__create__" => self.handle_create(&event).await,
                "__dispose__" => self.handle_dispose(&event).await,
                "__adopt__" => self.handle_adopt(&event).await,
                _ => match self.objects.lock().get(&event.guid).cloned() {
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
                },
            },
        }
    }

    async fn handle_create(self: &Arc<Self>, event: &Event) -> Result<()> {
        use crate::server::channel_owner::ParentOrConnection;
        use crate::server::object_factory::create_object;

        let type_name = event.params["type"]
            .as_str()
            .ok_or_else(|| Error::ProtocolError("__create__ missing 'type'".to_string()))?
            .to_string();

        let object_guid: Arc<str> = Arc::from(
            event.params["guid"]
                .as_str()
                .ok_or_else(|| Error::ProtocolError("__create__ missing 'guid'".to_string()))?,
        );

        tracing::debug!(
            "DEBUG __create__: type={}, guid={}, parent_guid={}",
            type_name,
            object_guid,
            event.guid
        );

        let initializer = event.params["initializer"].clone();

        let parent_or_conn = if event.guid.is_empty() {
            ParentOrConnection::Connection(Arc::clone(self) as Arc<dyn ConnectionLike>)
        } else {
            let parent_obj = self
                .objects
                .lock()
                .get(&event.guid)
                .cloned()
                .ok_or_else(|| {
                    tracing::error!(
                        "DEBUG: Parent object not found for type={}, parent_guid={}",
                        type_name,
                        event.guid
                    );
                    Error::ProtocolError(format!("Parent object not found: {}", event.guid))
                })?;
            ParentOrConnection::Parent(parent_obj)
        };

        let object = match create_object(
            parent_or_conn.clone(),
            type_name.clone(),
            object_guid.clone(),
            initializer,
        )
        .await
        {
            Ok(obj) => obj,
            Err(e) => {
                tracing::error!(
                    "DEBUG: Failed to create object type={}, guid={}, error={}",
                    type_name,
                    object_guid,
                    e
                );
                return Err(e);
            }
        };

        self.register_object(Arc::clone(&object_guid), object.clone())
            .await;

        if let ParentOrConnection::Parent(parent) = &parent_or_conn {
            parent.add_child(Arc::clone(&object_guid), object);
        }

        tracing::debug!(
            "DEBUG: Successfully created and registered object: type={}, guid={}",
            type_name,
            object_guid
        );

        Ok(())
    }

    async fn handle_dispose(self: &Arc<Self>, event: &Event) -> Result<()> {
        use crate::server::channel_owner::DisposeReason;

        let guid = &event.guid;

        // Find object - check lock in scope
        let object = { self.objects.lock().get(guid).cloned() };

        if let Some(object) = object {
            // Unregister from connection
            self.unregister_object(guid).await;

            // Remove from parent
            if let Some(parent) = object.parent() {
                parent.remove_child(guid);
            }

            // Dispose object
            object.dispose(DisposeReason::Protocol);

            tracing::debug!("Disposed object: guid={}", guid);
        } else {
            tracing::debug!("Ignoring __dispose__ for unknown object: guid={}", guid);
        }

        Ok(())
    }

    pub async fn wait_for_object(&self, guid: &str) -> Result<Arc<dyn ChannelOwner>> {
        use tokio::time::{Duration, sleep};
        let start = std::time::Instant::now();
        loop {
            if let Some(obj) = self.objects.lock().get(guid) {
                return Ok(obj.clone());
            }
            if start.elapsed() > Duration::from_secs(30) {
                return Err(Error::Timeout(format!(
                    "Timed out waiting for object {}",
                    guid
                )));
            }
            sleep(Duration::from_millis(10)).await;
        }
    }

    async fn handle_adopt(self: &Arc<Self>, event: &Event) -> Result<()> {
        let child_guid = event.params["guid"]
            .as_str()
            .ok_or_else(|| Error::ProtocolError("__adopt__ missing 'guid'".to_string()))?;

        let new_parent_guid = &event.guid;

        let child = self.get_object(child_guid).await.map_err(|e| {
            Error::ProtocolError(format!("Child object not found during adopt: {}", e))
        })?;

        let new_parent = self.get_object(new_parent_guid).await.map_err(|e| {
            Error::ProtocolError(format!("Parent object not found during adopt: {}", e))
        })?;

        // 1. Remove from old parent
        if let Some(old_parent) = child.parent() {
            old_parent.remove_child(child_guid);
        }

        // 2. Add to new parent (this will update child's weak parent ref if we had mutable access,
        // but since we only have Arc<dyn ChannelOwner>, ChannelOwner logic needs to handle it.
        // Actually, ChannelOwner trait has `adopt` method.
        new_parent.adopt(child);

        Ok(())
    }
}

impl ConnectionLike for Connection {
    fn send_message(
        &self,
        guid: &str,
        method: &str,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>> {
        let guid = guid.to_string();
        let method = method.to_string();
        Box::pin(self.send_message(guid, method, params))
    }

    fn register_object(
        &self,
        guid: Arc<str>,
        object: Arc<dyn ChannelOwner>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let objects = self.objects.clone();
        Box::pin(async move {
            objects.lock().insert(guid, object);
        })
    }

    fn unregister_object(&self, guid: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let objects = self.objects.clone();
        let guid = Arc::from(guid);
        Box::pin(async move {
            objects.lock().remove(&guid);
        })
    }

    fn get_object(&self, guid: &str) -> AsyncChannelOwnerResult<'_> {
        let guid = Arc::from(guid);
        Box::pin(async move {
            self.objects.lock().get(&guid).cloned().ok_or_else(|| {
                Error::ObjectNotFound(format!(
                    "{} (object may have been closed or disposed)",
                    guid
                ))
            })
        })
    }
}

/// Detects if an error message indicates a browser installation issue
fn is_browser_installation_error(message: &str) -> bool {
    message.contains("Looks like Playwright")
        || message.contains("Executable doesn't exist")
        || message.contains("not installed")
        || message.contains("Please run")
}

/// Extracts browser name from error message
fn extract_browser_name(message: &str) -> &str {
    // Check in priority order (specific to generic)
    if message.contains("chromium") {
        "chromium"
    } else if message.contains("firefox") {
        "firefox"
    } else if message.contains("webkit") {
        "webkit"
    } else {
        // If we can't detect the browser, use a generic message
        "browsers"
    }
}

fn parse_protocol_error(payload: ErrorPayload) -> Error {
    // Detect browser installation errors
    // Playwright server sends errors with messages like:
    // "Looks like Playwright Test or Playwright was just installed or updated."
    // or "browserType.launch: Executable doesn't exist at /path/to/chromium"

    let message = &payload.message;

    // Check for browser installation errors
    if is_browser_installation_error(message) {
        let browser_name = extract_browser_name(message);

        return Error::BrowserNotInstalled {
            browser_name: browser_name.to_string(),
            message: message.clone(),
            playwright_version: crate::PLAYWRIGHT_VERSION.to_string(),
        };
    }

    // Default: return as protocol error
    Error::ProtocolError(format!(
        "{} \n {}",
        payload.message,
        payload.stack.unwrap_or_default()
    ))
}
