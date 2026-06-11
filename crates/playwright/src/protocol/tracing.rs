// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Tracing — Playwright trace recording
//
// Architecture Reference:
// - Python: playwright-python/playwright/_impl/_tracing.py
// - JavaScript: playwright/packages/playwright-core/src/client/tracing.ts
// - Docs: https://playwright.dev/docs/api/class-tracing

//! Tracing — record Playwright traces for debugging
//!
//! Tracing is a per-context feature. Access the Tracing object via
//! [`BrowserContext::tracing`](crate::protocol::BrowserContext::tracing).
//!
//! # Example
//!
//! ```ignore
//! use playwright_rs::protocol::{Playwright, TracingStartOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let context = browser.new_context().await?;
//!
//!     let tracing = context.tracing()?;
//!
//!     // Start tracing with options
//!     tracing.start(Some(TracingStartOptions::default()
//!         .name("my-trace")
//!         .screenshots(true)
//!         .snapshots(true))).await?;
//!
//!     let page = context.new_page().await?;
//!     page.goto("https://example.com", None).await?;
//!
//!     // Stop and save the trace
//!     use playwright_rs::protocol::TracingStopOptions;
//!     tracing.stop(Some(TracingStopOptions::default()
//!         .path("/tmp/trace.zip"))).await?;
//!
//!     context.close().await?;
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-tracing>

use crate::error::Result;
use crate::protocol::har_options::StartHarOptions;
use crate::server::channel::Channel;
use crate::server::channel_owner::{
    ChannelOwner, ChannelOwnerImpl, DisposeReason, ParentOrConnection,
};
use crate::server::connection::ConnectionLike;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// Options for starting a trace recording.
///
/// See: <https://playwright.dev/docs/api/class-tracing#tracing-start>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TracingStartOptions {
    /// Custom name for the trace. Shown in trace viewer as the trace title.
    pub name: Option<String>,
    /// Whether to capture screenshots during tracing. Screenshots are used as
    /// a timeline preview in the trace viewer.
    pub screenshots: Option<bool>,
    /// Whether to capture DOM snapshots on each action.
    pub snapshots: Option<bool>,
    /// Whether to enable live trace updates while recording. When `true`,
    /// the trace viewer can attach and observe the trace as it is being
    /// captured, rather than waiting for the recording to finish. Useful
    /// for debugging long-running flows.
    ///
    /// See: <https://playwright.dev/docs/api/class-tracing#tracing-start-option-live>
    pub live: Option<bool>,
}

impl TracingStartOptions {
    /// Trace name (affects file naming in the traces directory).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    /// Capture screenshots during tracing.
    pub fn screenshots(mut self, screenshots: bool) -> Self {
        self.screenshots = Some(screenshots);
        self
    }
    /// Capture DOM snapshots during tracing.
    pub fn snapshots(mut self, snapshots: bool) -> Self {
        self.snapshots = Some(snapshots);
        self
    }
    /// Enable live tracing (view in the trace viewer while running).
    pub fn live(mut self, live: bool) -> Self {
        self.live = Some(live);
        self
    }
}

/// Options for stopping a trace recording.
///
/// See: <https://playwright.dev/docs/api/class-tracing#tracing-stop>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TracingStopOptions {
    /// Path to export the trace file to. If not provided, the trace is discarded.
    /// The file is written as a `.zip` archive.
    pub path: Option<String>,
}

impl TracingStopOptions {
    /// Export the trace to the given path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

/// In-flight HAR recording state, captured by `start_har` for `stop_har`.
struct HarRecording {
    har_id: Option<String>,
    path: String,
    resources_dir: Option<String>,
}

/// Tracing — records Playwright traces for debugging and inspection.
///
/// Trace files can be opened in the Playwright Trace Viewer.
/// This is a Chromium-only feature; calling tracing methods on Firefox or
/// WebKit contexts will fail.
///
/// See: <https://playwright.dev/docs/api/class-tracing>
#[derive(Clone)]
pub struct Tracing {
    base: ChannelOwnerImpl,
    /// Shared across clones so `start_har`/`stop_har` on the same context's
    /// `Tracing` see one recording. `stop_har` takes no path (matching the
    /// upstream API), so the path and `harId` are stashed here at start.
    har: Arc<parking_lot::Mutex<Option<HarRecording>>>,
}

impl Tracing {
    /// Creates a new Tracing from protocol initialization.
    ///
    /// Called by the object factory when the server sends a `__create__` message.
    pub fn new(
        parent: ParentOrConnection,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        Ok(Self {
            base: ChannelOwnerImpl::new(parent, type_name, guid, initializer),
            har: Arc::new(parking_lot::Mutex::new(None)),
        })
    }

    /// Start tracing.
    ///
    /// Playwright implements tracing as a two-step process: `tracingStart` to
    /// configure the trace, then `tracingStartChunk` to begin recording.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional trace configuration (name, screenshots, snapshots)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Tracing is already active
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-tracing#tracing-start>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn start(&self, options: Option<TracingStartOptions>) -> Result<()> {
        let opts = options.unwrap_or_default();

        // Step 1: tracingStart — configure the trace
        let mut start_params = serde_json::json!({});
        if let Some(ref name) = opts.name {
            start_params["name"] = serde_json::Value::String(name.clone());
        }
        if let Some(screenshots) = opts.screenshots {
            start_params["screenshots"] = serde_json::Value::Bool(screenshots);
        }
        if let Some(snapshots) = opts.snapshots {
            start_params["snapshots"] = serde_json::Value::Bool(snapshots);
        }
        if let Some(live) = opts.live {
            start_params["live"] = serde_json::Value::Bool(live);
        }

        self.channel()
            .send_no_result("tracingStart", start_params)
            .await?;

        // Step 2: tracingStartChunk — begin the chunk/recording
        let mut chunk_params = serde_json::json!({});
        if let Some(name) = opts.name {
            chunk_params["name"] = serde_json::Value::String(name);
        }

        self.channel()
            .send_no_result("tracingStartChunk", chunk_params)
            .await
    }

    /// Stop tracing.
    ///
    /// Playwright implements stopping as a two-step process: `tracingStopChunk`
    /// to finalize the recording, then `tracingStop` to tear down.
    ///
    /// If `options.path` is provided, the trace is exported to that file as a
    /// `.zip` archive. If no path is provided, the trace is discarded.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional stop options; set `path` to save the trace to a file
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Tracing was not active
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-tracing#tracing-stop>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn stop(&self, options: Option<TracingStopOptions>) -> Result<()> {
        let path = options.and_then(|o| o.path);

        // Step 1: tracingStopChunk — mode "entries" collects trace data
        // mode "archive" or "compressedTrace" would export, but "entries" is simpler
        let mode = if path.is_some() { "archive" } else { "discard" };
        let stop_chunk_params = serde_json::json!({ "mode": mode });

        let chunk_result: Value = self
            .channel()
            .send("tracingStopChunk", stop_chunk_params)
            .await?;

        // Step 2: tracingStop — tear down
        self.channel()
            .send_no_result("tracingStop", serde_json::json!({}))
            .await?;

        // If a path was requested, save the artifact
        if let Some(dest_path) = path
            && let Some(artifact_guid) = chunk_result
                .get("artifact")
                .and_then(|a| a.get("guid"))
                .and_then(|g| g.as_str())
        {
            // Resolve the artifact and save it
            self.save_artifact(artifact_guid, &dest_path).await?;
        }

        Ok(())
    }

    /// Save a trace artifact to a file path.
    async fn save_artifact(&self, artifact_guid: &str, dest_path: &str) -> Result<()> {
        use crate::protocol::artifact::Artifact;
        use crate::server::connection::ConnectionExt;

        let artifact = self
            .connection()
            .get_typed::<Artifact>(artifact_guid)
            .await?;

        artifact.save_as(dest_path).await
    }

    /// Start recording a HAR (HTTP Archive) of network traffic to `path`.
    ///
    /// The HAR is written when [`stop_har`](Self::stop_har) is called. A `.zip`
    /// path bundles resource bodies as separate entries (`Attach`); a plain
    /// path inlines them (`Embed`). The recorded HAR can be opened in browser
    /// devtools or replayed in tests via `route_from_har`.
    ///
    /// # Errors
    ///
    /// Returns an error if communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-tracing#tracing-start-har>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn start_har(
        &self,
        path: impl Into<String>,
        options: Option<StartHarOptions>,
    ) -> Result<()> {
        let path = path.into();
        let opts = options.unwrap_or_default();
        let rec_options = opts.to_record_har_json(&path);

        let result: Value = self
            .channel()
            .send("harStart", serde_json::json!({ "options": rec_options }))
            .await?;
        let har_id = result
            .get("harId")
            .and_then(|v| v.as_str())
            .map(str::to_owned);

        *self.har.lock() = Some(HarRecording {
            har_id,
            path,
            resources_dir: opts.resources_dir,
        });
        Ok(())
    }

    /// Stop the HAR recording started by [`start_har`](Self::start_har) and
    /// write it to the path given there.
    ///
    /// # Errors
    ///
    /// Returns an error if `start_har` was not called first, or if
    /// communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-tracing#tracing-stop-har>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn stop_har(&self) -> Result<()> {
        let Some(recording) = self.har.lock().take() else {
            return Err(crate::error::Error::InvalidArgument(
                "stop_har called without a matching start_har".to_string(),
            ));
        };

        let mut params = serde_json::json!({ "mode": "archive" });
        if let Some(id) = &recording.har_id {
            params["harId"] = Value::String(id.clone());
        }

        let result: Value = self.channel().send("harExport", params).await?;

        let Some(artifact_guid) = result
            .get("artifact")
            .and_then(|a| a.get("guid"))
            .and_then(|g| g.as_str())
        else {
            return Ok(());
        };

        // harExport always yields a zip archive. A `.zip` destination takes it
        // verbatim; any other path gets the `.har` JSON extracted out of it.
        if recording.path.ends_with(".zip") {
            self.save_artifact(artifact_guid, &recording.path).await?;
        } else {
            let tmp_zip = format!("{}.tmp.zip", recording.path);
            self.save_artifact(artifact_guid, &tmp_zip).await?;
            let local_utils = self.find_local_utils()?;
            local_utils
                .har_unzip(
                    &tmp_zip,
                    &recording.path,
                    recording.resources_dir.as_deref(),
                )
                .await?;
            let _ = std::fs::remove_file(&tmp_zip);
        }

        Ok(())
    }

    /// Locate the connection's `LocalUtils` (used to extract a `.har` from the
    /// exported zip archive).
    fn find_local_utils(&self) -> Result<crate::protocol::LocalUtils> {
        let connection = self.connection();
        connection
            .all_objects_sync()
            .into_iter()
            .find(|o| o.type_name() == "LocalUtils")
            .and_then(|o| {
                o.as_any()
                    .downcast_ref::<crate::protocol::LocalUtils>()
                    .cloned()
            })
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "stop_har: LocalUtils not found in connection registry".to_string(),
                )
            })
    }
}

impl ChannelOwner for Tracing {
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

impl std::fmt::Debug for Tracing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tracing")
            .field("guid", &self.guid())
            .finish()
    }
}
