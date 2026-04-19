// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Video protocol object
//
// Represents a video recording associated with a page.
// Video recording is enabled via BrowserContextOptions::record_video.

use crate::error::{Error, Result};
use crate::server::channel_owner::ChannelOwner;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Video represents a video recording of a page.
///
/// Video recording is enabled by passing `record_video` to
/// `Browser::new_context_with_options()`. Each page in the context receives
/// its own `Video` object accessible via `page.video()`.
///
/// The underlying recording is backed by an `Artifact` whose GUID is provided
/// in the `Page` initializer. Methods that access the file wait for the
/// artifact to become ready before acting — in practice this happens almost
/// immediately, but calling `path()` or `save_as()` before the page is closed
/// may return an error if the artifact hasn't finished writing.
///
/// See: <https://playwright.dev/docs/api/class-video>
#[derive(Clone)]
pub struct Video {
    /// Shared state: the artifact once the "video" event fires, or an error if
    /// the page was closed without producing frames.
    inner: Arc<VideoInner>,
}

struct VideoInner {
    /// Mutex-protected artifact slot; populated by `set_artifact`.
    artifact: Mutex<Option<Arc<dyn ChannelOwner>>>,
    /// Notifier for waiters: incremented whenever `artifact` is set.
    notify: tokio::sync::Notify,
}

impl Video {
    /// Creates a new `Video` shell with no artifact resolved yet.
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(VideoInner {
                artifact: Mutex::new(None),
                notify: tokio::sync::Notify::new(),
            }),
        }
    }

    /// Called once the artifact GUID has been resolved via the connection.
    pub(crate) fn set_artifact(&self, artifact: Arc<dyn ChannelOwner>) {
        let mut guard = self.inner.artifact.lock().unwrap();
        *guard = Some(artifact);
        drop(guard);
        self.inner.notify.notify_waiters();
    }

    /// Waits for the artifact to become available, then returns its channel.
    ///
    /// Polls up to ~10 seconds before giving up, matching typical Playwright timeouts.
    async fn wait_for_artifact_channel(&self) -> Result<crate::server::channel::Channel> {
        const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
        const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

        let deadline = tokio::time::Instant::now() + TIMEOUT;

        loop {
            // Check if already available
            {
                let guard = self.inner.artifact.lock().unwrap();
                if let Some(artifact) = guard.as_ref() {
                    return Ok(artifact.channel().clone());
                }
            }

            if tokio::time::Instant::now() >= deadline {
                return Err(Error::ProtocolError(
                    "Video artifact not available after 10 seconds. \
                     Close the page before calling video methods to ensure the \
                     recording is finalised."
                        .to_string(),
                ));
            }

            // Wait for notification or poll interval, whichever comes first
            tokio::select! {
                _ = self.inner.notify.notified() => {}
                _ = tokio::time::sleep(POLL_INTERVAL) => {}
            }
        }
    }

    /// Returns the file system path of the video recording.
    ///
    /// The recording is guaranteed to be written to the filesystem after the
    /// browser context closes. This method waits up to 10 seconds for the
    /// recording to be ready.
    ///
    /// See: <https://playwright.dev/docs/api/class-video#video-path>
    pub async fn path(&self) -> Result<std::path::PathBuf> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PathResponse {
            value: String,
        }

        let channel = self.wait_for_artifact_channel().await?;
        let resp: PathResponse = channel
            .send("pathAfterFinished", serde_json::json!({}))
            .await?;
        Ok(std::path::PathBuf::from(resp.value))
    }

    /// Saves the video recording to the specified path.
    ///
    /// This method can be called while recording is still in progress, or after
    /// the page has been closed. It waits up to 10 seconds for the recording to
    /// be ready.
    ///
    /// See: <https://playwright.dev/docs/api/class-video#video-save-as>
    pub async fn save_as(&self, path: impl AsRef<Path>) -> Result<()> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::InvalidArgument("path contains invalid UTF-8".to_string()))?;

        let channel = self.wait_for_artifact_channel().await?;
        channel
            .send_no_result("saveAs", serde_json::json!({ "path": path_str }))
            .await
    }

    /// Deletes the video file.
    ///
    /// This method waits up to 10 seconds for the recording to finish before deleting.
    ///
    /// See: <https://playwright.dev/docs/api/class-video#video-delete>
    pub async fn delete(&self) -> Result<()> {
        let channel = self.wait_for_artifact_channel().await?;
        channel
            .send_no_result("delete", serde_json::json!({}))
            .await
    }
}

impl std::fmt::Debug for Video {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Video").finish()
    }
}
