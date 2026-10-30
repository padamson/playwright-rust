//! Error types for trace parsing.

use std::io;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TraceError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// The archive's local file headers disagree with its central
    /// directory, or its entries repeat a name or overlap. Two zip readers
    /// can see different contents in such an archive, so the trace is
    /// refused rather than parsed from whichever view this one would read.
    #[error("trace rejected, archive headers are inconsistent: {0}")]
    InconsistentArchive(#[source] zip::result::ZipError),

    #[error("json on line {line}: {source}")]
    Json {
        line: usize,
        #[source]
        source: serde_json::Error,
    },

    #[error("missing entry: {0}")]
    MissingEntry(&'static str),

    #[error("unsupported trace version {found}, supported {supported:?}")]
    UnsupportedVersion {
        found: u32,
        supported: std::ops::RangeInclusive<u32>,
    },

    #[error("malformed action {call_id}: {reason}")]
    MalformedAction { call_id: String, reason: String },
}

pub type Result<T> = std::result::Result<T, TraceError>;
