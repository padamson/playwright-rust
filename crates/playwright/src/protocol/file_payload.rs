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
/// let file = FilePayload::new("document.pdf", "application/pdf", vec![/* PDF bytes */]);
/// ```
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FilePayload {
    /// File name
    pub name: String,
    /// MIME type
    pub mime_type: String,
    /// File contents as bytes
    pub buffer: Vec<u8>,
}

use crate::error::Result;
use std::fs;
use std::path::Path;

impl FilePayload {
    /// Creates a FilePayload from a name, MIME type, and contents.
    pub fn new(name: impl Into<String>, mime_type: impl Into<String>, buffer: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            mime_type: mime_type.into(),
            buffer,
        }
    }

    /// Creates a FilePayload from a file path.
    ///
    /// Automatically detects the MIME type based on the file extension.
    /// Reads the file into memory.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let name = path
            .file_name()
            .ok_or_else(|| crate::Error::InvalidPath(format!("Path {:?} has no filename", path)))?
            .to_string_lossy()
            .into_owned();

        let mime_type = crate::protocol::mime::from_path(path).to_string();
        let buffer = fs::read(path)?;

        Ok(Self {
            name,
            mime_type,
            buffer,
        })
    }

    /// Creates a FilePayload from a file path with an explicit MIME type.
    pub fn from_file<P: AsRef<Path>>(path: P, mime_type: &str) -> Result<Self> {
        let path = path.as_ref();
        let name = path
            .file_name()
            .ok_or_else(|| crate::Error::InvalidPath(format!("Path {:?} has no filename", path)))?
            .to_string_lossy()
            .into_owned();

        let buffer = fs::read(path)?;

        Ok(Self {
            name,
            mime_type: mime_type.to_string(),
            buffer,
        })
    }
}
