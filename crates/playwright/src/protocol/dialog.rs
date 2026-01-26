// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Dialog protocol object
//
// Represents a browser dialog (alert, confirm, prompt, or beforeunload)
// dispatched via page.on('dialog') events.

use crate::error::Result;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde_json::{Value, json};
use std::any::Any;
use std::sync::Arc;

/// Dialog represents a browser dialog (alert, confirm, prompt, or beforeunload).
///
/// Dialogs are dispatched via the page.on('dialog') event. Dialogs must be
/// explicitly accepted or dismissed, otherwise the page will freeze waiting
/// for the dialog to be handled.
///
/// See module-level documentation for usage examples.
///
/// See: <https://playwright.dev/docs/api/class-dialog>
#[derive(Clone)]
pub struct Dialog {
    base: ChannelOwnerImpl,
}

impl Dialog {
    /// Creates a new Dialog from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Dialog object.
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

    /// Returns the dialog's type.
    ///
    /// Returns one of:
    /// - "alert" - Simple notification dialog
    /// - "confirm" - Yes/No confirmation dialog
    /// - "prompt" - Text input dialog
    /// - "beforeunload" - Page unload confirmation dialog
    ///
    /// See: <https://playwright.dev/docs/api/class-dialog#dialog-type>
    pub fn type_(&self) -> &str {
        self.initializer()
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Returns the message displayed in the dialog.
    ///
    /// See: <https://playwright.dev/docs/api/class-dialog#dialog-message>
    pub fn message(&self) -> &str {
        self.initializer()
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Returns the default value for prompt dialogs.
    ///
    /// For prompt dialogs, returns the default input value.
    /// For other dialog types (alert, confirm, beforeunload), returns an empty string.
    ///
    /// See: <https://playwright.dev/docs/api/class-dialog#dialog-default-value>
    pub fn default_value(&self) -> &str {
        self.initializer()
            .get("defaultValue")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Accepts the dialog.
    ///
    /// For prompt dialogs, optionally provides text input.
    /// For other dialog types, the promptText parameter is ignored.
    ///
    /// # Arguments
    ///
    /// * `prompt_text` - Optional text to enter in a prompt dialog
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Dialog has already been accepted or dismissed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-dialog#dialog-accept>
    pub async fn accept(&self, prompt_text: Option<&str>) -> Result<()> {
        let params = if let Some(text) = prompt_text {
            json!({ "promptText": text })
        } else {
            json!({})
        };

        self.channel().send_no_result("accept", params).await?;

        Ok(())
    }

    /// Dismisses the dialog.
    ///
    /// For confirm dialogs, this is equivalent to clicking "Cancel".
    /// For prompt dialogs, this is equivalent to clicking "Cancel".
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Dialog has already been accepted or dismissed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-dialog#dialog-dismiss>
    pub async fn dismiss(&self) -> Result<()> {
        self.channel().send_no_result("dismiss", json!({})).await?;

        Ok(())
    }
}

impl ChannelOwner for Dialog {
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
        // Dialog doesn't emit events
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Dialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("guid", &self.guid())
            .field("type", &self.type_())
            .field("message", &self.message())
            .field("default_value", &self.default_value())
            .finish()
    }
}
