// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0

use crate::error::Result;
use crate::server::channel::Channel;
use crate::server::channel_owner::{
    ChannelOwner, ChannelOwnerImpl, DisposeReason, ParentOrConnection,
};
use crate::server::connection::ConnectionLike;
use base64::Engine;
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// LocalUtils protocol object
///
/// Provides client-side utility operations: HAR file replay, zip, tracing helpers.
#[derive(Clone)]
pub struct LocalUtils {
    base: ChannelOwnerImpl,
}

impl LocalUtils {
    pub fn new(
        parent: ParentOrConnection,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        Ok(Self {
            base: ChannelOwnerImpl::new(parent, type_name, guid, initializer),
        })
    }

    /// Opens a HAR file and returns an opaque HAR ID for subsequent lookups.
    ///
    /// Sends `"harOpen"` RPC with the file path.
    pub async fn har_open(&self, file: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct HarOpenResult {
            #[serde(rename = "harId")]
            har_id: String,
        }
        let result: HarOpenResult = self
            .channel()
            .send("harOpen", serde_json::json!({ "file": file }))
            .await?;
        Ok(result.har_id)
    }

    /// Looks up a request in the opened HAR archive.
    ///
    /// Returns an action object describing how the request should be fulfilled,
    /// redirected, or allowed to fall through.
    pub async fn har_lookup(
        &self,
        har_id: &str,
        url: &str,
        method: &str,
        headers: Vec<serde_json::Value>,
        post_data: Option<&[u8]>,
        is_navigation_request: bool,
    ) -> Result<HarLookupResult> {
        let mut params = serde_json::json!({
            "harId": har_id,
            "url": url,
            "method": method,
            "headers": headers,
            "isNavigationRequest": is_navigation_request,
        });

        // Only include postData when present — the server rejects null.
        if let Some(data) = post_data {
            let encoded = base64::engine::general_purpose::STANDARD.encode(data);
            params["postData"] = serde_json::Value::String(encoded);
        }

        self.channel().send("harLookup", params).await
    }

    /// Closes a previously opened HAR archive.
    pub async fn har_close(&self, har_id: &str) -> Result<()> {
        self.channel()
            .send_no_result("harClose", serde_json::json!({ "harId": har_id }))
            .await
    }
}

/// Result from a `harLookup` RPC call.
///
/// Describes whether the request was found in the HAR and how to respond.
#[derive(Debug, Deserialize)]
pub struct HarLookupResult {
    /// Action to take: `"fulfill"`, `"redirect"`, `"fallback"`, or `"error"`.
    pub action: String,

    /// For `"redirect"`: the URL to redirect to.
    #[serde(rename = "redirectURL")]
    pub redirect_url: Option<String>,

    /// For `"fulfill"`: HTTP status code.
    pub status: Option<u16>,

    /// For `"fulfill"`: HTTP headers as `[{"name": ..., "value": ...}]`.
    pub headers: Option<Vec<serde_json::Value>>,

    /// For `"fulfill"`: Base64-encoded response body.
    pub body: Option<String>,
}

impl ChannelOwner for LocalUtils {
    fn guid(&self) -> &str {
        self.base.guid()
    }

    fn type_name(&self) -> &str {
        self.base.type_name()
    }

    fn parent(&self) -> Option<Arc<dyn ChannelOwner>> {
        self.base.parent()
    }

    fn connection(&self) -> Arc<dyn ConnectionLike> {
        self.base.connection()
    }

    fn initializer(&self) -> &Value {
        self.base.initializer()
    }

    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    fn dispose(&self, reason: DisposeReason) {
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

    fn on_event(&self, method: &str, params: Value) {
        self.base.on_event(method, params)
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for LocalUtils {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalUtils")
            .field("guid", &self.guid())
            .finish()
    }
}
