// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Channel Owner - Base trait for all Playwright protocol objects
//
// Architecture Reference:
// - Python: playwright-python/playwright/_impl/_connection.py (ChannelOwner class)
// - Java: playwright-java/.../impl/ChannelOwner.java
// - JavaScript: playwright/.../client/channelOwner.ts
//
// All Playwright objects (Browser, Page, etc.) implement ChannelOwner to:
// - Represent remote objects on the server via GUID
// - Participate in parent-child lifecycle management
// - Handle protocol events
// - Communicate via Channel proxy

use crate::server::channel::Channel;
use crate::server::connection::ConnectionLike;
use parking_lot::Mutex;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

/// Type alias for the children registry mapping GUIDs to ChannelOwner objects
type ChildrenRegistry = HashMap<Arc<str>, Arc<dyn ChannelOwner>>;

/// Reason why an object was disposed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisposeReason {
    /// Object was explicitly closed by user code
    Closed,
    /// Object was garbage collected by the server
    GarbageCollected,
    /// Object was disposed by the protocol
    Protocol,
}

/// Parent can be either another ChannelOwner or the root Connection
#[derive(Clone)]
pub enum ParentOrConnection {
    Parent(Arc<dyn ChannelOwner>),
    Connection(Arc<dyn ConnectionLike>),
}

/// Base trait for all Playwright protocol objects.
///
/// Every object in the Playwright protocol (Browser, Page, BrowserContext, etc.)
/// implements this trait to enable:
/// - GUID-based object identity and lookup
/// - Hierarchical parent-child lifecycle management
/// - Channel-based RPC communication
/// - Protocol event handling
///
/// # Architecture
///
/// All official Playwright bindings (Python, Java, .NET) follow this pattern:
///
/// 1. **GUID Identity**: Each object has a unique GUID from the server
/// 2. **Parent-Child Tree**: Objects form a hierarchy (e.g., Browser → BrowserContext → Page)
/// 3. **Dual Registry**: Objects are registered in both connection (global) and parent (lifecycle)
/// 4. **Channel Communication**: Objects send/receive messages via their Channel
/// 5. **Event Handling**: Protocol events are dispatched to objects by GUID
///
/// # Example
///
/// ```ignore
/// # use playwright_rs::server::channel_owner::ChannelOwner;
/// # use std::sync::Arc;
/// # fn example(browser: Arc<dyn ChannelOwner>) {
/// // Get object identity
/// println!("Object GUID: {}", browser.guid());
/// println!("Object type: {}", browser.type_name());
///
/// // Handle lifecycle
/// browser.dispose(playwright_rs::server::channel_owner::DisposeReason::Closed);
/// # }
/// ```
pub trait ChannelOwner: Send + Sync {
    /// Returns the unique GUID for this object.
    ///
    /// The GUID is assigned by the Playwright server and used for:
    /// - Looking up objects in the connection registry
    /// - Routing protocol messages to the correct object
    /// - Parent-child relationship tracking
    fn guid(&self) -> &str;

    /// Returns the protocol type name (e.g., "Browser", "Page").
    fn type_name(&self) -> &str;

    /// Returns the parent object, if any.
    ///
    /// The root Playwright object has no parent.
    fn parent(&self) -> Option<Arc<dyn ChannelOwner>>;

    /// Returns the connection this object belongs to.
    fn connection(&self) -> Arc<dyn ConnectionLike>;

    /// Returns the raw initializer JSON from the server.
    ///
    /// The initializer contains the object's initial state sent
    /// in the `__create__` protocol message.
    fn initializer(&self) -> &Value;

    /// Returns the channel for RPC communication.
    fn channel(&self) -> &Channel;

    /// Disposes this object and all its children.
    ///
    /// Called when:
    /// - Server sends `__dispose__` message
    /// - User explicitly closes the object
    /// - Parent is disposed (cascades to children)
    ///
    /// # Arguments
    /// * `reason` - Why the object is being disposed
    fn dispose(&self, reason: DisposeReason);

    /// Adopts a child object (moves from old parent to this parent).
    ///
    /// Called when server sends `__adopt__` message, typically when:
    /// - A page is moved between browser contexts
    /// - An object's ownership changes
    fn adopt(&self, child: Arc<dyn ChannelOwner>);

    /// Adds a child object to this parent's registry.
    ///
    /// Called during object creation and adoption.
    fn add_child(&self, guid: Arc<str>, child: Arc<dyn ChannelOwner>);

    /// Removes a child object from this parent's registry.
    ///
    /// Called during disposal and adoption.
    fn remove_child(&self, guid: &str);

    /// Handles a protocol event sent to this object.
    ///
    /// # Arguments
    /// * `method` - Event name (e.g., "close", "load")
    /// * `params` - Event parameters as JSON
    fn on_event(&self, method: &str, params: Value);

    /// Returns true if this object was garbage collected.
    fn was_collected(&self) -> bool;

    /// Enables downcasting to concrete types.
    ///
    /// Required for converting `Arc<dyn ChannelOwner>` to specific types
    /// like `Arc<Browser>` when retrieving objects from the connection.
    fn as_any(&self) -> &dyn Any;
}

/// Base implementation of ChannelOwner that can be embedded in protocol objects.
///
/// This struct provides the common functionality for all ChannelOwner implementations.
/// Protocol objects (Browser, Page, etc.) should contain this as a field and
/// delegate trait methods to it.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection, DisposeReason};
/// use playwright_rs::server::channel::Channel;
/// use playwright_rs::server::connection::ConnectionLike;
/// use std::sync::Arc;
/// use std::any::Any;
/// use serde_json::Value;
///
/// pub struct Browser {
///     base: ChannelOwnerImpl,
///     // ... browser-specific fields
/// }
///
/// impl Browser {
///     pub fn new(
///         parent: Arc<dyn ChannelOwner>,
///         type_name: String,
///         guid: Arc<str>,
///         initializer: Value,
///     ) -> Self {
///         let base = ChannelOwnerImpl::new(
///             ParentOrConnection::Parent(parent),
///             type_name,
///             guid,
///             initializer,
///         );
///         Self { base }
///     }
/// }
///
/// impl ChannelOwner for Browser {
///     fn guid(&self) -> &str { self.base.guid() }
///     fn type_name(&self) -> &str { self.base.type_name() }
///     fn parent(&self) -> Option<Arc<dyn ChannelOwner>> { self.base.parent() }
///     fn connection(&self) -> Arc<dyn ConnectionLike> { self.base.connection() }
///     fn initializer(&self) -> &Value { self.base.initializer() }
///     fn channel(&self) -> &Channel { self.base.channel() }
///     fn dispose(&self, reason: DisposeReason) { self.base.dispose(reason) }
///     fn adopt(&self, child: Arc<dyn ChannelOwner>) { self.base.adopt(child) }
///     fn add_child(&self, guid: Arc<str>, child: Arc<dyn ChannelOwner>) {
///         self.base.add_child(guid, child)
///     }
///     fn remove_child(&self, guid: &str) { self.base.remove_child(guid) }
///     fn on_event(&self, method: &str, params: Value) { self.base.on_event(method, params) }
///     fn was_collected(&self) -> bool { self.base.was_collected() }
///     fn as_any(&self) -> &dyn Any { self }
/// }
/// ```
pub struct ChannelOwnerImpl {
    guid: Arc<str>,
    type_name: String,
    parent: Option<Weak<dyn ChannelOwner>>,
    connection: Arc<dyn ConnectionLike>,
    children: Arc<Mutex<ChildrenRegistry>>,
    channel: Channel,
    initializer: Value,
    was_collected: AtomicBool,
}

impl Clone for ChannelOwnerImpl {
    fn clone(&self) -> Self {
        Self {
            guid: self.guid.clone(),
            type_name: self.type_name.clone(),
            parent: self.parent.clone(),
            connection: Arc::clone(&self.connection),
            children: Arc::clone(&self.children),
            channel: self.channel.clone(),
            initializer: self.initializer.clone(),
            was_collected: AtomicBool::new(
                self.was_collected.load(std::sync::atomic::Ordering::SeqCst),
            ),
        }
    }
}

impl ChannelOwnerImpl {
    /// Creates a new ChannelOwner base implementation.
    ///
    /// This constructor:
    /// 1. Extracts the connection from parent or uses provided connection
    /// 2. Creates the channel for RPC communication
    /// 3. Stores the initializer data
    /// 4. Registers itself in the connection (done by caller via Connection::register_object)
    /// 5. Registers itself in parent (done by caller via parent.add_child)
    ///
    /// # Arguments
    /// * `parent` - Either a parent ChannelOwner or the root Connection
    /// * `type_name` - Protocol type name (e.g., "Browser")
    /// * `guid` - Unique GUID from server
    /// * `initializer` - Initial state from `__create__` message
    pub fn new(
        parent: ParentOrConnection,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Self {
        let (connection, parent_opt) = match parent {
            ParentOrConnection::Parent(p) => {
                let conn = p.connection();
                (conn, Some(Arc::downgrade(&p)))
            }
            ParentOrConnection::Connection(c) => (c, None),
        };

        // Arc<str> allows efficient sharing of GUID without cloning
        let channel = Channel::new(Arc::clone(&guid), connection.clone());

        Self {
            guid,
            type_name,
            parent: parent_opt,
            connection,
            children: Arc::new(Mutex::new(HashMap::new())),
            channel,
            initializer,
            was_collected: AtomicBool::new(false),
        }
    }

    /// Returns the unique GUID for this object.
    pub fn guid(&self) -> &str {
        &self.guid
    }

    /// Returns the protocol type name.
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Returns the parent object, if any.
    pub fn parent(&self) -> Option<Arc<dyn ChannelOwner>> {
        self.parent.as_ref().and_then(|p| p.upgrade())
    }

    /// Returns the connection.
    pub fn connection(&self) -> Arc<dyn ConnectionLike> {
        self.connection.clone()
    }

    /// Returns the initializer JSON.
    pub fn initializer(&self) -> &Value {
        &self.initializer
    }

    /// Returns the channel for RPC.
    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    /// Disposes this object and all children recursively.
    ///
    /// # Arguments
    /// * `reason` - Why the object is being disposed
    pub fn dispose(&self, reason: DisposeReason) {
        // Mark as collected if garbage collected
        if reason == DisposeReason::GarbageCollected {
            self.was_collected.store(true, Ordering::SeqCst);
        }

        // Remove from parent
        if let Some(parent) = self.parent() {
            parent.remove_child(&self.guid);
        }

        // Remove from connection (spawn to avoid blocking in sync context)
        let connection = self.connection.clone();
        let guid = Arc::clone(&self.guid);
        tokio::spawn(async move {
            connection.unregister_object(&guid).await;
        });

        // Dispose all children (snapshot to avoid holding lock)
        let children: Vec<_> = {
            let guard = self.children.lock();
            guard.values().cloned().collect()
        };

        for child in children {
            child.dispose(reason);
        }

        // Clear children
        self.children.lock().clear();
    }

    /// Adopts a child object (moves from old parent to this parent).
    pub fn adopt(&self, child: Arc<dyn ChannelOwner>) {
        // Remove from old parent
        if let Some(old_parent) = child.parent() {
            old_parent.remove_child(child.guid());
        }

        // Add to this parent
        // Convert &str to Arc<str> for the hashmap key
        self.add_child(Arc::from(child.guid()), child);
    }

    /// Adds a child to this parent's registry.
    pub fn add_child(&self, guid: Arc<str>, child: Arc<dyn ChannelOwner>) {
        self.children.lock().insert(guid, child);
    }

    /// Removes a child from this parent's registry.
    pub fn remove_child(&self, guid: &str) {
        // Create Arc<str> for lookup
        let guid_arc: Arc<str> = Arc::from(guid);
        self.children.lock().remove(&guid_arc);
    }

    /// Handles a protocol event (default implementation logs it).
    ///
    /// Subclasses should override this to handle specific events.
    pub fn on_event(&self, method: &str, params: Value) {
        tracing::debug!(
            "Event on {} ({}): {} -> {:?}",
            self.guid,
            self.type_name,
            method,
            params
        );
    }

    /// Returns true if object was garbage collected.
    pub fn was_collected(&self) -> bool {
        self.was_collected.load(Ordering::SeqCst)
    }
}

// Note: ChannelOwner testing is done via integration tests since it requires:
// - Real Connection with object registry
// - Multiple connected objects (parent-child relationships)
// - Protocol messages from the server
// See: crates/playwright-core/tests/connection_integration.rs
