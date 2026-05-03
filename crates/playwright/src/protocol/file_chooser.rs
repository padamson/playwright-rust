// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// FileChooser - plain data struct constructed from "fileChooser" event params.
//
// FileChooser is NOT a ChannelOwner. It is constructed directly from
// the event params when a "fileChooser" event is received on the Page channel.
// The event params contain an ElementHandle GUID and an isMultiple flag.
//
// See: <https://playwright.dev/docs/api/class-filechooser>

use crate::error::Result;
use crate::protocol::ElementHandle;
use std::path::PathBuf;
use std::sync::Arc;

/// Represents a file chooser dialog triggered by an `<input type="file">` element.
///
/// `FileChooser` objects are dispatched by the `"fileChooser"` event on
/// [`Page`](crate::protocol::Page) via [`on_filechooser`](crate::protocol::Page::on_filechooser)
/// and [`expect_file_chooser`](crate::protocol::Page::expect_file_chooser).
///
/// # Usage
///
/// Use [`set_files`](FileChooser::set_files) to satisfy the file chooser by providing
/// file paths. The files are read from disk and sent to the browser.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::Playwright;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let browser = playwright.chromium().launch().await?;
///     let page = browser.new_page().await?;
///
///     page.set_content(
///         r#"<input type="file" id="f" />"#,
///         None
///     ).await?;
///
///     // Set up the waiter BEFORE the action that triggers the file chooser
///     let waiter = page.expect_file_chooser(None).await?;
///
///     // Click the file input to open the chooser
///     page.locator("#f").await.click(None).await?;
///
///     // Resolve the waiter and set files
///     let chooser = waiter.wait().await?;
///     chooser.set_files(&[PathBuf::from("/tmp/test.txt")]).await?;
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-filechooser>
#[derive(Clone)]
pub struct FileChooser {
    /// Back-reference to the page that owns this file chooser
    page: crate::protocol::Page,
    /// The `<input type="file">` element that triggered the chooser
    element: Arc<ElementHandle>,
    /// Whether the input accepts multiple files
    is_multiple: bool,
}

impl FileChooser {
    /// Creates a new `FileChooser` from event params.
    ///
    /// Called by `Page::on_event("fileChooser")` when a fileChooser event
    /// is received from the Playwright server.
    pub(crate) fn new(
        page: crate::protocol::Page,
        element: Arc<ElementHandle>,
        is_multiple: bool,
    ) -> Self {
        Self {
            page,
            element,
            is_multiple,
        }
    }

    /// Returns the page that owns this file chooser.
    ///
    /// See: <https://playwright.dev/docs/api/class-filechooser#file-chooser-page>
    pub fn page(&self) -> &crate::protocol::Page {
        &self.page
    }

    /// Returns the `<input type="file">` element that triggered this chooser.
    ///
    /// See: <https://playwright.dev/docs/api/class-filechooser#file-chooser-element>
    pub fn element(&self) -> Arc<ElementHandle> {
        self.element.clone()
    }

    /// Returns `true` if the file input accepts multiple files.
    ///
    /// This reflects the `multiple` attribute on the underlying `<input type="file">`.
    ///
    /// See: <https://playwright.dev/docs/api/class-filechooser#file-chooser-is-multiple>
    pub fn is_multiple(&self) -> bool {
        self.is_multiple
    }

    /// Sets files on the associated `<input type="file">` element.
    ///
    /// Reads each file from disk, encodes as base64, and sends a `setInputFiles`
    /// RPC directly on the ElementHandle channel. This satisfies the file chooser
    /// dialog without requiring any OS-level file picker interaction.
    ///
    /// # Arguments
    ///
    /// * `files` - Slice of file paths to set on the input element
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Any file cannot be read from disk
    /// - The RPC call to the browser fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use std::path::PathBuf;
    /// let waiter = page.expect_file_chooser(None).await?;
    /// page.locator("input[type=file]").await.click(None).await?;
    /// let chooser = waiter.wait().await?;
    /// chooser.set_files(&[PathBuf::from("/tmp/upload.txt")]).await?;
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-filechooser#file-chooser-set-files>
    #[tracing::instrument(level = "debug", skip_all, fields(count = files.len()))]
    pub async fn set_files(&self, files: &[PathBuf]) -> Result<()> {
        self.element.set_input_files(files).await
    }
}

impl std::fmt::Debug for FileChooser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileChooser")
            .field("is_multiple", &self.is_multiple)
            .finish()
    }
}
