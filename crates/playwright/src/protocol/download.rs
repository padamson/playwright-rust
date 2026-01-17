// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Download protocol object
//
// Represents a file download triggered by the page.
// Downloads are dispatched via page.on('download') events.

use crate::error::Result;
use crate::server::channel_owner::ChannelOwner;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

/// Download represents a file download triggered by the page.
///
/// Downloads are dispatched via the page.on('download') event.
/// The download will be automatically deleted when the browser context closes.
///
/// NOTE: Unlike other protocol objects, Download is a wrapper around the Artifact
/// protocol object. The URL and suggested_filename come from the download event params,
/// while the actual file operations are delegated to the underlying Artifact.
///
/// See: <https://playwright.dev/docs/api/class-download>
#[derive(Clone)]
pub struct Download {
    /// Reference to the underlying Artifact protocol object
    artifact: Arc<dyn ChannelOwner>,
    /// URL from download event params
    url: String,
    /// Suggested filename from download event params
    suggested_filename: String,
}

impl Download {
    /// Creates a new Download from an Artifact and event params
    ///
    /// This is NOT created via the object factory, but rather constructed
    /// directly from the download event params which contain {url, suggestedFilename, artifact}.
    ///
    /// # Arguments
    ///
    /// * `artifact` - The Artifact protocol object (from event params)
    /// * `url` - Download URL (from event params)
    /// * `suggested_filename` - Suggested filename (from event params)
    pub fn from_artifact(
        artifact: Arc<dyn ChannelOwner>,
        url: String,
        suggested_filename: String,
    ) -> Self {
        Self {
            artifact,
            url,
            suggested_filename,
        }
    }

    /// Returns the download URL.
    ///
    /// See: <https://playwright.dev/docs/api/class-download#download-url>
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the suggested filename for the download.
    ///
    /// This is typically the server-suggested filename from the Content-Disposition
    /// header or the HTML download attribute.
    ///
    /// See: <https://playwright.dev/docs/api/class-download#download-suggested-filename>
    pub fn suggested_filename(&self) -> &str {
        &self.suggested_filename
    }

    /// Returns the underlying Artifact's channel for protocol communication
    fn channel(&self) -> &crate::server::channel::Channel {
        self.artifact.channel()
    }

    /// Returns the path to the downloaded file after it completes.
    ///
    /// This method waits for the download to finish if necessary.
    /// Returns an error if the download fails or is canceled.
    ///
    /// See: <https://playwright.dev/docs/api/class-download#download-path>
    pub async fn path(&self) -> Result<Option<PathBuf>> {
        #[derive(serde::Deserialize)]
        struct PathResponse {
            value: Option<String>,
        }

        let result: PathResponse = self.channel().send("path", json!({})).await?;

        Ok(result.value.map(PathBuf::from))
    }

    /// Saves the download to the specified path.
    ///
    /// This method can be safely called while the download is still in progress.
    /// The file will be copied to the specified location after the download completes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use playwright_rs::protocol::Download;
    /// # async fn example(download: Download) -> Result<(), Box<dyn std::error::Error>> {
    /// download.save_as("/path/to/save/file.pdf").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-download#download-save-as>
    pub async fn save_as(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| crate::error::Error::InvalidArgument("Invalid path".to_string()))?;

        self.channel()
            .send_no_result("saveAs", json!({ "path": path_str }))
            .await?;

        Ok(())
    }

    /// Cancels the download.
    ///
    /// After calling this method, `failure()` will return an error message.
    ///
    /// See: <https://playwright.dev/docs/api/class-download#download-cancel>
    pub async fn cancel(&self) -> Result<()> {
        self.channel().send_no_result("cancel", json!({})).await?;

        Ok(())
    }

    /// Deletes the downloaded file.
    ///
    /// The download must be finished before calling this method.
    ///
    /// See: <https://playwright.dev/docs/api/class-download#download-delete>
    pub async fn delete(&self) -> Result<()> {
        self.channel().send_no_result("delete", json!({})).await?;

        Ok(())
    }

    /// Returns the download error message if it failed, otherwise None.
    ///
    /// See: <https://playwright.dev/docs/api/class-download#download-failure>
    pub async fn failure(&self) -> Result<Option<String>> {
        #[derive(serde::Deserialize)]
        struct FailureResponse {
            error: Option<String>,
        }

        let result: FailureResponse = self.channel().send("failure", json!({})).await?;

        Ok(result.error)
    }
}

impl std::fmt::Debug for Download {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Download")
            .field("url", &self.url())
            .field("suggested_filename", &self.suggested_filename())
            .finish()
    }
}
