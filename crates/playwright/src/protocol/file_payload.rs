// FilePayload protocol type
//
// Represents a file to be uploaded with explicit name, MIME type, and buffer.
//
// See: https://playwright.dev/docs/api/class-locator#locator-set-input-files

/// FilePayload represents a file for advanced file uploads.
///
/// Allows explicit control over filename, MIME type, and file contents
/// when uploading files to input elements.
///
/// # Example
///
/// ```ignore
/// # use playwright_rs::protocol::FilePayload;
/// let file = FilePayload::builder()
///     .name("document.pdf".to_string())
///     .mime_type("application/pdf".to_string())
///     .buffer(vec![/* PDF bytes */])
///     .build();
/// ```
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
#[derive(Debug, Clone)]
pub struct FilePayload {
    /// File name
    pub name: String,
    /// MIME type
    pub mime_type: String,
    /// File contents as bytes
    pub buffer: Vec<u8>,
}

impl FilePayload {
    /// Creates a new builder for FilePayload
    pub fn builder() -> FilePayloadBuilder {
        FilePayloadBuilder::default()
    }
}

/// Builder for FilePayload
#[derive(Debug, Clone, Default)]
pub struct FilePayloadBuilder {
    name: Option<String>,
    mime_type: Option<String>,
    buffer: Option<Vec<u8>>,
}

impl FilePayloadBuilder {
    /// Sets the file name
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets the MIME type
    pub fn mime_type(mut self, mime_type: String) -> Self {
        self.mime_type = Some(mime_type);
        self
    }

    /// Sets the file buffer (contents as bytes)
    pub fn buffer(mut self, buffer: Vec<u8>) -> Self {
        self.buffer = Some(buffer);
        self
    }

    /// Builds the FilePayload
    ///
    /// # Panics
    ///
    /// Panics if any required field (name, mime_type, buffer) is missing
    pub fn build(self) -> FilePayload {
        FilePayload {
            name: self.name.expect("name is required for FilePayload"),
            mime_type: self
                .mime_type
                .expect("mime_type is required for FilePayload"),
            buffer: self.buffer.expect("buffer is required for FilePayload"),
        }
    }
}
