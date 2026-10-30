//! [`TraceReader`] — open a Playwright trace zip and stream its
//! contents lazily.

use crate::action::{Action, ActionStream};
use crate::error::{Result, TraceError};
use crate::event::{ContextOptions, RawEvent, TraceEvent};
use crate::jsonl::JsonLines;
use crate::network::NetworkEntry;
use std::io::{BufRead, BufReader, Read, Seek};
use std::path::Path;
use zip::ZipArchive;
use zip::result::ZipError;

const TRACE_ENTRY: &str = "trace.trace";
const NETWORK_ENTRY: &str = "trace.network";
/// Trace formats this parser reads. v9 gave frame snapshots a `phase`
/// and wrote blob references as whole archive paths; all formats parse,
/// and a reference from any opens through [`TraceReader::blob`]. v10
/// dropped `stepId` and takes call ids from the client (`abcd@7`, where
/// v9 wrote `call@7`), which changes no field this parser types.
const SUPPORTED_VERSIONS: std::ops::RangeInclusive<u32> = 8..=10;
const RESOURCE_SNAPSHOT_KIND: &str = "resource-snapshot";

/// Streaming reader over a Playwright trace zip.
///
/// Opens the archive and parses the first event (`context-options`)
/// eagerly so the trace's metadata is available without consuming the
/// rest of the stream. Subsequent calls to
/// [`raw_events`](Self::raw_events), [`events`](Self::events), or
/// [`actions`](Self::actions) iterate the remaining events lazily;
/// each call extracts a fresh JSONL stream from the archive, so the
/// reader can be iterated multiple times.
pub struct TraceReader<R: Read + Seek> {
    zip: ZipArchive<R>,
    context: ContextOptions,
}

impl<R: Read + Seek> TraceReader<R> {
    /// Open a trace from any `Read + Seek` source. For the typical
    /// file-on-disk case prefer [`crate::open`].
    ///
    /// Every entry's local header is checked against the central directory
    /// before anything is parsed. The check covers the archive as it stands
    /// at open; later reads from the same source are not checked again.
    ///
    /// # Errors
    ///
    /// - [`TraceError::Zip`] when the source is not a readable zip archive.
    /// - [`TraceError::InconsistentArchive`] when its headers disagree, or
    ///   its entries repeat a name or overlap.
    /// - [`TraceError::MissingEntry`] when it has no `trace.trace`, or that
    ///   entry holds no events.
    /// - [`TraceError::Json`] or [`TraceError::MalformedAction`] when the
    ///   first event is not valid `context-options` JSON.
    /// - [`TraceError::UnsupportedVersion`] for a trace format outside 8 to 10.
    /// - [`TraceError::Io`] when reading the source fails.
    pub fn open(reader: R) -> Result<Self> {
        let mut zip = ZipArchive::new(reader)?;
        zip.validate_header_consistency()
            .map_err(consistency_error)?;
        let context = parse_context(&mut zip)?;
        if !SUPPORTED_VERSIONS.contains(&context.version) {
            return Err(TraceError::UnsupportedVersion {
                found: context.version,
                supported: SUPPORTED_VERSIONS,
            });
        }
        Ok(Self { zip, context })
    }

    /// The `context-options` metadata from the trace's first event.
    pub fn context(&self) -> &ContextOptions {
        &self.context
    }

    /// Lossless stream of every JSONL event in `trace.trace`. Yields a
    /// [`RawEvent`] per line; callers can dispatch on
    /// [`RawEvent::kind`](crate::RawEvent::kind) to handle event types
    /// the typed enum doesn't model.
    ///
    /// The first event (`context-options`) is **included** in the
    /// stream; if you only need it, [`context`](Self::context) is
    /// already cached.
    pub fn raw_events(&mut self) -> Result<impl Iterator<Item = Result<RawEvent>>> {
        let entry = self.zip.by_name(TRACE_ENTRY)?;
        let lines = JsonLines::new(BufReader::new(entry));
        Ok(lines.map(|res| res.map(RawEvent::new)))
    }

    /// Typed stream of events. Wraps [`raw_events`](Self::raw_events)
    /// and routes each [`RawEvent`] through
    /// [`RawEvent::into_typed`](crate::RawEvent::into_typed). Unknown
    /// or unmodelled kinds surface as [`TraceEvent::Unknown`].
    pub fn events(&mut self) -> Result<impl Iterator<Item = Result<TraceEvent>>> {
        Ok(self
            .raw_events()?
            .map(|res| res.map(|raw| raw.into_typed())))
    }

    /// Reassembled action stream — `before` + optional `input` + zero-
    /// or-more `log` + `after` events sharing a `call_id` are merged
    /// into one [`Action`].
    ///
    /// Actions are yielded in `after`-arrival order, **not** strictly
    /// in `start_time` order — concurrent calls can interleave.
    /// Callers wanting chronological order should collect into a
    /// `Vec` and sort by [`Action::start_time`](crate::Action::start_time).
    ///
    /// Truncated actions (no matching `after` event, e.g. a trace cut
    /// short by a crash) are emitted at end-of-stream with
    /// `end_time = None` rather than discarded.
    pub fn actions(&mut self) -> Result<impl Iterator<Item = Result<Action>>> {
        Ok(ActionStream::new(self.events()?))
    }

    /// The bytes of a blob the trace refers to: a screencast frame, a
    /// response body, a snapshot resource. `path` is the archive path an
    /// event carries in its `file` field.
    ///
    /// # Errors
    ///
    /// Returns [`TraceError::Zip`] when the archive holds no such entry.
    pub fn blob(&mut self, path: &str) -> Result<Vec<u8>> {
        let mut entry = self.zip.by_name(path)?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// Streaming iterator over [`NetworkEntry`] records from
    /// `trace.network`. Yields zero items when the trace recorded no
    /// requests (the entry is present but empty).
    ///
    /// HAR fields not modelled on [`NetworkEntry`] are preserved on
    /// [`NetworkEntry::raw_snapshot`].
    pub fn network(&mut self) -> Result<impl Iterator<Item = Result<NetworkEntry>>> {
        let entry = self.zip.by_name(NETWORK_ENTRY)?;
        let lines = JsonLines::new(BufReader::new(entry));
        Ok(lines.map(|res| {
            let mut map = res?;
            // Check the discriminator before deserialising the
            // payload — otherwise serde rejects an unexpected kind
            // with a confusing "missing field `snapshot`" message.
            let kind = map
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if kind != RESOURCE_SNAPSHOT_KIND {
                return Err(TraceError::MalformedAction {
                    call_id: String::new(),
                    reason: format!(
                        "trace.network: expected `{RESOURCE_SNAPSHOT_KIND}` event, got `{kind}`",
                    ),
                });
            }
            let snapshot = map
                .remove("snapshot")
                .ok_or_else(|| TraceError::MalformedAction {
                    call_id: String::new(),
                    reason: "trace.network: resource-snapshot missing `snapshot` payload".into(),
                })?;
            NetworkEntry::from_snapshot(snapshot)
                .map_err(|source| TraceError::Json { line: 0, source })
        }))
    }
}

/// An I/O failure during the consistency pass says nothing about the
/// archive's headers, so it surfaces like any other failed zip read.
fn consistency_error(error: ZipError) -> TraceError {
    match error {
        ZipError::Io(_) => TraceError::Zip(error),
        other => TraceError::InconsistentArchive(other),
    }
}

fn parse_context<R: Read + Seek>(zip: &mut ZipArchive<R>) -> Result<ContextOptions> {
    let entry = zip
        .by_name(TRACE_ENTRY)
        .map_err(|_| TraceError::MissingEntry(TRACE_ENTRY))?;
    let mut reader = BufReader::new(entry);
    let mut line = String::new();
    let mut line_no = 0;

    loop {
        line.clear();
        line_no += 1;
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Err(TraceError::MissingEntry(TRACE_ENTRY));
        }
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed.trim().is_empty() {
            continue;
        }

        let value: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|source| TraceError::Json {
                line: line_no,
                source,
            })?;

        let kind = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if kind != "context-options" {
            return Err(TraceError::MalformedAction {
                call_id: String::new(),
                reason: format!("expected first event to be `context-options`, got `{kind}`"),
            });
        }

        return serde_json::from_value::<ContextOptions>(value).map_err(|source| {
            TraceError::Json {
                line: line_no,
                source,
            }
        });
    }
}

/// Convenience wrapper for [`TraceReader::open`] over a file on disk.
pub fn open<P: AsRef<Path>>(path: P) -> Result<TraceReader<std::fs::File>> {
    let file = std::fs::File::open(path)?;
    TraceReader::open(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_io_failure_during_the_consistency_pass_is_not_a_rejection() {
        let error = consistency_error(ZipError::Io(std::io::Error::other("disk gone")));
        assert!(
            matches!(error, TraceError::Zip(ZipError::Io(_))),
            "got {error:?}"
        );
    }

    #[test]
    fn a_header_mismatch_is_a_rejection() {
        let error = consistency_error(ZipError::InvalidArchive("mismatch".into()));
        assert!(
            matches!(
                error,
                TraceError::InconsistentArchive(ZipError::InvalidArchive(_))
            ),
            "got {error:?}"
        );
    }
}
