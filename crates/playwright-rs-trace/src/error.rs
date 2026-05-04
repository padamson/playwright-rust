//! Error types for trace parsing.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum TraceError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("json on line {line}: {source}")]
    Json {
        line: usize,
        #[source]
        source: serde_json::Error,
    },

    #[error("missing entry: {0}")]
    MissingEntry(&'static str),

    #[error("unsupported trace version {found}, expected {expected}")]
    UnsupportedVersion { found: u32, expected: u32 },

    #[error("malformed action {call_id}: {reason}")]
    MalformedAction { call_id: String, reason: String },
}

pub type Result<T> = std::result::Result<T, TraceError>;
