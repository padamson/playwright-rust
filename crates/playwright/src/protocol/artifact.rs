// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Artifact protocol object
//
// Artifacts represent downloaded files. The Artifact is wrapped by
// the Download class which adds URL and filename from event params.

use crate::error::Result;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// Artifact is the protocol object for downloaded files.
///
/// NOTE: This is an internal protocol object. Users interact with Download objects,
/// which wrap Artifact and include metadata from download events.
#[derive(Clone)]
pub struct Artifact {
    base: ChannelOwnerImpl,
}

impl Artifact {
    /// Creates a new Artifact from protocol initialization
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
}

impl ChannelOwner for Artifact {
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
        // Artifact doesn't emit events
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Artifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Artifact")
            .field("guid", &self.guid())
            .finish()
    }
}
