// Request protocol object
//
// Represents an HTTP request. Created during navigation operations.
// In Playwright's architecture, navigation creates a Request which receives a Response.

use crate::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::error::Result;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// Request represents an HTTP request during navigation.
///
/// Request objects are created by the server during navigation operations.
/// They are parents to Response objects.
///
/// See: <https://playwright.dev/docs/api/class-request>
#[derive(Clone)]
pub struct Request {
    base: ChannelOwnerImpl,
}

impl Request {
    /// Creates a new Request from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Request object.
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: String,
        initializer: Value,
    ) -> Result<Self> {
        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer,
        );

        Ok(Self { base })
    }
}

impl ChannelOwner for Request {
    fn guid(&self) -> &str {
        self.base.guid()
    }

    fn type_name(&self) -> &str {
        self.base.type_name()
    }

    fn parent(&self) -> Option<Arc<dyn ChannelOwner>> {
        self.base.parent()
    }

    fn connection(&self) -> Arc<dyn crate::connection::ConnectionLike> {
        self.base.connection()
    }

    fn initializer(&self) -> &Value {
        self.base.initializer()
    }

    fn channel(&self) -> &crate::channel::Channel {
        self.base.channel()
    }

    fn dispose(&self, reason: crate::channel_owner::DisposeReason) {
        self.base.dispose(reason)
    }

    fn adopt(&self, child: Arc<dyn ChannelOwner>) {
        self.base.adopt(child)
    }

    fn add_child(&self, guid: String, child: Arc<dyn ChannelOwner>) {
        self.base.add_child(guid, child)
    }

    fn remove_child(&self, guid: &str) {
        self.base.remove_child(guid)
    }

    fn on_event(&self, _method: &str, _params: Value) {
        // Request events will be handled in future phases
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("guid", &self.guid())
            .finish()
    }
}
