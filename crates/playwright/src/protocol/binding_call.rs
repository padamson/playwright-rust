// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// BindingCall protocol object
//
// Represents a single invocation of a binding registered via expose_function
// or expose_binding. When JavaScript code calls the exposed function, the
// Playwright server creates a BindingCall object and fires a "bindingCall"
// event on the BrowserContext.
//
// The BindingCall must be resolved (via "fulfill") or rejected (via "reject")
// to unblock the JS caller.
//
// See: https://playwright.dev/docs/api/class-browsercontext#browser-context-expose-function

use crate::error::Result;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// BindingCall represents a single JS → Rust callback invocation.
///
/// When JavaScript calls an exposed function (registered via `expose_function`
/// or `expose_binding`), the server sends a `bindingCall` event on the
/// BrowserContext channel containing the GUID of a freshly created BindingCall
/// object. The Rust handler must call either [`resolve`](BindingCall::resolve)
/// or [`reject`](BindingCall::reject) to unblock the JS caller.
///
/// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-expose-function>
#[derive(Clone)]
pub struct BindingCall {
    base: ChannelOwnerImpl,
}

impl BindingCall {
    /// Creates a new BindingCall from protocol initialization.
    ///
    /// Called by the object factory when the server sends a `__create__`
    /// message for a BindingCall object.
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: Arc<str>,
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

    /// Returns the name of the binding that was called.
    ///
    /// Matches the `name` argument passed to `expose_function` / `expose_binding`.
    pub fn name(&self) -> &str {
        self.base
            .initializer()
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Returns the raw serialized arguments sent by the JS caller.
    ///
    /// This is the `args` array from the initializer, in Playwright's
    /// type-tagged protocol format (e.g. `[{"n": 3}, {"n": 7}]`).
    pub fn args(&self) -> &Value {
        self.base.initializer().get("args").unwrap_or(&Value::Null)
    }

    /// Resolves the binding call with a result value.
    ///
    /// Sends the `resolve` RPC back to the Playwright server so the
    /// JS `await` of the exposed function resolves with `result`.
    ///
    /// # Arguments
    ///
    /// * `result` - The value to return to the JavaScript caller, already
    ///   serialized in Playwright's `serialize_argument` format
    ///   (`{"value": ..., "handles": []}`).
    ///
    /// # Errors
    ///
    /// Returns error if communication with the browser process fails.
    pub async fn resolve(&self, result: Value) -> Result<()> {
        self.base
            .channel()
            .send_no_result("resolve", serde_json::json!({ "result": result }))
            .await
    }

    /// Rejects the binding call with an error.
    ///
    /// Sends the `reject` RPC so the JS `await` of the exposed function
    /// rejects with the given error message.
    ///
    /// # Arguments
    ///
    /// * `message` - Human-readable error description sent to the JS caller.
    ///
    /// # Errors
    ///
    /// Returns error if communication with the browser process fails.
    pub async fn reject(&self, message: &str) -> Result<()> {
        self.base
            .channel()
            .send_no_result(
                "reject",
                serde_json::json!({ "error": { "error": { "message": message, "name": "Error" } } }),
            )
            .await
    }
}

impl ChannelOwner for BindingCall {
    fn guid(&self) -> &str {
        self.base.guid()
    }

    fn type_name(&self) -> &str {
        self.base.type_name()
    }

    fn parent(&self) -> Option<Arc<dyn ChannelOwner>> {
        self.base.parent()
    }

    fn connection(&self) -> Arc<dyn crate::server::connection::ConnectionLike> {
        self.base.connection()
    }

    fn initializer(&self) -> &Value {
        self.base.initializer()
    }

    fn channel(&self) -> &crate::server::channel::Channel {
        self.base.channel()
    }

    fn dispose(&self, reason: crate::server::channel_owner::DisposeReason) {
        self.base.dispose(reason)
    }

    fn adopt(&self, child: Arc<dyn ChannelOwner>) {
        self.base.adopt(child)
    }

    fn add_child(&self, guid: Arc<str>, child: Arc<dyn ChannelOwner>) {
        self.base.add_child(guid, child)
    }

    fn remove_child(&self, guid: &str) {
        self.base.remove_child(guid)
    }

    fn on_event(&self, _method: &str, _params: Value) {
        // BindingCall does not emit events
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for BindingCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindingCall")
            .field("guid", &self.guid())
            .field("name", &self.name())
            .finish()
    }
}
