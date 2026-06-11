// DropOptions and related types
//
// Provides configuration for Locator::drop, matching Playwright's API.

use crate::protocol::FilePayload;
use crate::protocol::click::Position;

/// Options for [`Locator::drop()`](crate::protocol::Locator::drop).
///
/// Simulates an external drag-and-drop of files and/or data onto an element
/// (e.g. an upload drop zone), dispatching `dragenter`/`dragover`/`drop` with a
/// synthetic `DataTransfer`. This is distinct from
/// [`Locator::drag_to()`](crate::protocol::Locator::drag_to), which drags one
/// element onto another within the page.
///
/// Set `files` and/or `data` (the driver requires at least one).
///
/// # Example
///
/// ```ignore
/// use playwright_rs::{DropOptions, FilePayload, Position};
///
/// // Drop an in-memory file onto a drop zone
/// let file = FilePayload::new("note.txt", "text/plain", b"hello".to_vec());
/// let options = DropOptions::builder().file(file).build();
///
/// // Drop MIME-typed data (e.g. a dragged URL) at a specific point
/// let options = DropOptions::builder()
///     .data("text/uri-list", "https://example.com")
///     .position(Position { x: 10.0, y: 10.0 })
///     .build();
/// ```
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-drop>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct DropOptions {
    /// In-memory files to drop (serialized to the protocol's `payloads`).
    pub files: Vec<FilePayload>,
    /// MIME-typed data entries to drop, as `(mime_type, value)` pairs.
    pub data: Vec<(String, String)>,
    /// Point within the element to drop at (relative to its top-left corner).
    pub position: Option<Position>,
    /// Maximum time in milliseconds.
    pub timeout: Option<f64>,
}

impl DropOptions {
    /// Create a new builder for DropOptions
    pub fn builder() -> DropOptionsBuilder {
        DropOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        use base64::{Engine as _, engine::general_purpose};

        let mut json = serde_json::json!({});

        if !self.files.is_empty() {
            let payloads: Vec<serde_json::Value> = self
                .files
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "name": f.name,
                        "mimeType": f.mime_type,
                        "buffer": general_purpose::STANDARD.encode(&f.buffer),
                    })
                })
                .collect();
            json["payloads"] = serde_json::Value::Array(payloads);
        }

        if !self.data.is_empty() {
            let data: Vec<serde_json::Value> = self
                .data
                .iter()
                .map(|(mime, value)| serde_json::json!({ "mimeType": mime, "value": value }))
                .collect();
            json["data"] = serde_json::Value::Array(data);
        }

        if let Some(position) = &self.position {
            json["position"] =
                serde_json::to_value(position).expect("serialization of position cannot fail");
        }

        // Timeout is required in Playwright 1.56.1+
        if let Some(timeout) = self.timeout {
            json["timeout"] = serde_json::json!(timeout);
        } else {
            json["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        json
    }
}

/// Builder for DropOptions
///
/// Provides a fluent API for constructing drop options.
#[derive(Debug, Clone, Default)]
pub struct DropOptionsBuilder {
    files: Vec<FilePayload>,
    data: Vec<(String, String)>,
    position: Option<Position>,
    timeout: Option<f64>,
}

impl DropOptionsBuilder {
    /// Add one in-memory file to drop.
    pub fn file(mut self, file: FilePayload) -> Self {
        self.files.push(file);
        self
    }

    /// Set the in-memory files to drop (replaces any already added).
    pub fn files(mut self, files: Vec<FilePayload>) -> Self {
        self.files = files;
        self
    }

    /// Add a MIME-typed data entry to drop (e.g. `"text/plain"` or `"text/uri-list"`).
    pub fn data(mut self, mime_type: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.push((mime_type.into(), value.into()));
        self
    }

    /// Set the point within the element to drop at (relative to top-left corner).
    pub fn position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    /// Set timeout in milliseconds.
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the DropOptions
    pub fn build(self) -> DropOptions {
        DropOptions {
            files: self.files,
            data: self.data,
            position: self.position,
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drop_options_default() {
        let json = DropOptions::builder().build().to_json();
        assert!(json["timeout"].is_number());
        assert!(json.get("payloads").is_none());
        assert!(json.get("data").is_none());
        assert!(json.get("position").is_none());
    }

    #[test]
    fn test_drop_options_file_payload() {
        let file = FilePayload::new("note.txt", "text/plain", b"hi".to_vec());
        let json = DropOptions::builder().file(file).build().to_json();
        assert_eq!(json["payloads"][0]["name"], "note.txt");
        assert_eq!(json["payloads"][0]["mimeType"], "text/plain");
        assert_eq!(json["payloads"][0]["buffer"], "aGk="); // base64("hi")
    }

    #[test]
    fn test_drop_options_data() {
        let json = DropOptions::builder()
            .data("text/uri-list", "https://example.com")
            .build()
            .to_json();
        assert_eq!(json["data"][0]["mimeType"], "text/uri-list");
        assert_eq!(json["data"][0]["value"], "https://example.com");
    }

    #[test]
    fn test_drop_options_position_and_timeout() {
        let json = DropOptions::builder()
            .position(Position { x: 10.0, y: 20.0 })
            .timeout(5000.0)
            .build()
            .to_json();
        assert_eq!(json["position"]["x"], 10.0);
        assert_eq!(json["position"]["y"], 20.0);
        assert_eq!(json["timeout"], 5000.0);
    }
}
