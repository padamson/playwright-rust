//! Trace event types.
//!
//! `RawEvent` is the lossless representation — every JSONL line in
//! `trace.trace` deserialises into one. `TraceEvent` is the typed
//! convenience layer; unknown / unmodelled kinds fall back to
//! `TraceEvent::Unknown(RawEvent)` so nothing is silently dropped.

use serde::Deserialize;
use serde_json::{Map, Value};

/// A single event from the trace, preserved as the underlying JSON
/// object. Forward-compat escape hatch for callers who need to dispatch
/// on event kinds the parser doesn't model yet.
#[derive(Debug, Clone)]
pub struct RawEvent {
    raw: Map<String, Value>,
}

impl RawEvent {
    pub(crate) fn new(raw: Map<String, Value>) -> Self {
        Self { raw }
    }

    /// Returns the value of the `"type"` field, or `None` if the event
    /// is malformed (`type` absent or non-string). The streaming
    /// iterators in [`crate::TraceReader`] filter out malformed events
    /// before they reach the user, so handlers iterating on
    /// [`TraceReader::raw_events`](crate::TraceReader::raw_events) can
    /// generally `expect` this.
    pub fn kind(&self) -> Option<&str> {
        self.raw.get("type").and_then(|v| v.as_str())
    }

    /// The full underlying JSON object, including the `"type"` field.
    pub fn as_value(&self) -> &Map<String, Value> {
        &self.raw
    }

    /// Take ownership of the underlying JSON.
    pub fn into_value(self) -> Value {
        Value::Object(self.raw)
    }

    /// Materialise the typed enum. Always succeeds — recognised kinds
    /// become typed variants; anything else (including known kinds
    /// whose schema we fail to deserialize) becomes
    /// [`TraceEvent::Unknown`].
    pub fn into_typed(self) -> TraceEvent {
        // Try to deserialize as the tagged enum. If it fails (unknown
        // tag, or a known tag with unexpected payload shape), preserve
        // the raw payload as `Unknown` rather than discarding it.
        // Borrowing deserialization: only the strings a variant keeps are
        // copied, and the map is handed back intact on a miss.
        let value = Value::Object(self.raw);
        match TypedEnum::deserialize(&value) {
            Ok(t) => t.into(),
            Err(_) => match value {
                Value::Object(raw) => TraceEvent::Unknown(RawEvent { raw }),
                _ => unreachable!("the value was built from an object above"),
            },
        }
    }
}

/// Strongly-typed variants for the event kinds this version of the
/// parser models. Unknown / unmodelled kinds surface as
/// [`TraceEvent::Unknown`] to preserve the underlying JSON.
#[derive(Debug, Clone)]
pub enum TraceEvent {
    ContextOptions(ContextOptions),
    Before(BeforeEvent),
    Input(InputEvent),
    Log(LogEvent),
    After(AfterEvent),
    Console(ConsoleEvent),
    Event(SystemEvent),
    FrameSnapshot(FrameSnapshotEvent),
    ScreencastFrame(ScreencastFrameEvent),
    /// Catch-all preserving the raw payload. Carries [`RawEvent`] so
    /// users keep full access to the JSON for kinds we don't model.
    Unknown(RawEvent),
}

// Internal enum used purely for serde-driven dispatch on the `type`
// field. Public callers always see `TraceEvent`.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum TypedEnum {
    ContextOptions(ContextOptions),
    Before(BeforeEvent),
    Input(InputEvent),
    Log(LogEvent),
    After(AfterEvent),
    Console(ConsoleEvent),
    Event(SystemEvent),
    /// The payload sits under a `snapshot` key.
    FrameSnapshot {
        snapshot: FrameSnapshotWire,
    },
    ScreencastFrame(ScreencastFrameEvent),
}

impl From<TypedEnum> for TraceEvent {
    fn from(t: TypedEnum) -> Self {
        match t {
            TypedEnum::ContextOptions(c) => TraceEvent::ContextOptions(c),
            TypedEnum::Before(b) => TraceEvent::Before(b),
            TypedEnum::Input(i) => TraceEvent::Input(i),
            TypedEnum::Log(l) => TraceEvent::Log(l),
            TypedEnum::After(a) => TraceEvent::After(a),
            TypedEnum::Console(c) => TraceEvent::Console(c),
            TypedEnum::Event(e) => TraceEvent::Event(e),
            TypedEnum::FrameSnapshot { snapshot } => TraceEvent::FrameSnapshot(snapshot.into()),
            TypedEnum::ScreencastFrame(s) => TraceEvent::ScreencastFrame(s),
        }
    }
}

/// Per-context metadata — appears once per trace as the first event
/// in `trace.trace`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextOptions {
    pub version: u32,
    #[serde(default)]
    pub browser_name: String,
    #[serde(default)]
    pub playwright_version: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub sdk_language: String,
    #[serde(default)]
    pub test_id_attribute_name: String,
    #[serde(default)]
    pub wall_time: f64,
    #[serde(default)]
    pub monotonic_time: f64,
    #[serde(default)]
    pub context_id: String,
    /// Original `options` blob, kept as raw JSON since its shape varies
    /// with browser type and Playwright version.
    #[serde(default)]
    pub options: Value,
}

/// Action-start event. Pairs with a matching [`AfterEvent`] sharing
/// `call_id`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeforeEvent {
    pub call_id: String,
    pub start_time: f64,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub title: Option<String>,
    pub page_id: Option<String>,
    pub before_snapshot: Option<String>,
    pub step_id: Option<String>,
    pub parent_id: Option<String>,
}

/// Optional input-coordinate / input-snapshot reference attached to an
/// in-flight action.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputEvent {
    pub call_id: String,
    pub point: Option<Point>,
    pub input_snapshot: Option<String>,
}

/// Log line emitted during an in-flight action.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub call_id: String,
    pub message: String,
    pub time: f64,
}

/// Action-completion event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterEvent {
    pub call_id: String,
    pub end_time: f64,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<ActionError>,
    pub after_snapshot: Option<String>,
    pub point: Option<Point>,
}

/// Browser console output captured during the trace.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleEvent {
    /// `"log"`, `"warn"`, `"error"`, `"info"`, `"debug"`, etc. Kept as
    /// a string because Playwright extends this set; matching at the
    /// call site keeps us forward-compatible.
    ///
    /// The driver writes this under `messageType`: `type` is the event's
    /// own discriminator, so reading it here left the level always empty.
    #[serde(rename = "messageType", default)]
    pub level: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub args: Vec<Value>,
    pub location: Option<ConsoleLocation>,
    pub time: f64,
    pub page_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleLocation {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub line_number: u32,
    #[serde(default)]
    pub column_number: u32,
}

/// System events (dialog, download, page open/close). Mirrors the
/// `event` chunk type in the trace.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEvent {
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub params: Value,
    pub time: f64,
    pub page_id: Option<String>,
}

/// Per-frame DOM snapshot. Includes the full DOM payload, which can be
/// sizeable; callers iterating on snapshots for many frames should expect
/// the per-event size to dominate the overall trace memory budget.
///
/// A snapshot belongs to the action whose `call_id` it names, at the
/// moment its `phase` says. Trace v9 writes the phase; v8 named each
/// snapshot (`before@<call>`) and the phase is read back from that name,
/// so the link is the same on both formats.
#[derive(Debug, Clone)]
pub struct FrameSnapshotEvent {
    pub call_id: String,
    /// The moment of the action this snapshot captured.
    pub phase: ActionPhase,
    /// The v8 snapshot name that the action's own events refer to. `None`
    /// on v9, which stopped naming snapshots.
    pub snapshot_name: Option<String>,
    pub page_id: String,
    pub frame_id: String,
    pub frame_url: String,
    pub doctype: String,
    /// The serialized DOM, in the trace viewer's nested-array encoding.
    pub html: Value,
    pub viewport: Option<Viewport>,
    pub timestamp: f64,
    pub wall_time: f64,
    pub collection_time: f64,
    pub is_main_frame: bool,
    pub resource_overrides: Vec<ResourceOverride>,
}

/// The `snapshot` payload as written, before the phase is settled.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSnapshotWire {
    call_id: String,
    #[serde(default)]
    phase: Option<ActionPhase>,
    #[serde(default)]
    snapshot_name: Option<String>,
    page_id: String,
    frame_id: String,
    #[serde(default)]
    frame_url: String,
    #[serde(default)]
    doctype: String,
    #[serde(default)]
    html: Value,
    viewport: Option<Viewport>,
    timestamp: f64,
    #[serde(default)]
    wall_time: f64,
    #[serde(default)]
    collection_time: f64,
    #[serde(default)]
    is_main_frame: bool,
    #[serde(default)]
    resource_overrides: Vec<ResourceOverride>,
}

impl From<FrameSnapshotWire> for FrameSnapshotEvent {
    fn from(wire: FrameSnapshotWire) -> Self {
        let phase = wire
            .phase
            .or_else(|| wire.snapshot_name.as_deref().map(ActionPhase::from_v8_name))
            .unwrap_or(ActionPhase::Other);
        Self {
            call_id: wire.call_id,
            phase,
            snapshot_name: wire.snapshot_name,
            page_id: wire.page_id,
            frame_id: wire.frame_id,
            frame_url: wire.frame_url,
            doctype: wire.doctype,
            html: wire.html,
            viewport: wire.viewport,
            timestamp: wire.timestamp,
            wall_time: wire.wall_time,
            collection_time: wire.collection_time,
            is_main_frame: wire.is_main_frame,
            resource_overrides: wire.resource_overrides,
        }
    }
}

/// The moment of an action a snapshot captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionPhase {
    /// Just before the action ran.
    Before,
    /// At the input, e.g. the click point.
    Action,
    /// After the action completed.
    After,
    /// A phase this parser does not know.
    #[serde(other)]
    Other,
}

impl ActionPhase {
    /// The phase a v8 snapshot name encodes: `before@<call>`,
    /// `input@<call>`, or `after@<call>`.
    fn from_v8_name(name: &str) -> Self {
        match name.split('@').next() {
            Some("before") => Self::Before,
            Some("input") => Self::Action,
            Some("after") => Self::After,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

/// External resource reference used by a snapshot. Either a blob in the
/// archive or an internal reference identifier the trace viewer reassembles.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceOverride {
    pub url: String,
    /// Path of the blob inside the archive, ready to open there. Trace v8
    /// wrote the entry name relative to `resources/`; v9 writes the whole
    /// path. Both are normalized to the path.
    #[serde(default, alias = "sha1", deserialize_with = "resource_path")]
    pub file: Option<String>,
    /// Ordinal of the earlier snapshot whose copy of this resource still
    /// applies, for a stylesheet the page mutated and then left alone.
    #[serde(rename = "ref", default)]
    pub reference: Option<u64>,
}

/// Single screencast frame stored as a JPEG in the archive.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastFrameEvent {
    pub page_id: String,
    /// Path of the JPEG inside the archive, ready to open there. Trace v9
    /// keeps frames under `screencast/`; v8 kept them under `resources/`
    /// and wrote only the entry name, which is normalized to the path.
    #[serde(alias = "sha1", deserialize_with = "required_resource_path")]
    pub file: String,
    pub width: u32,
    pub height: u32,
    pub timestamp: f64,
}

/// The archive directory every trace v8 blob reference was relative to.
/// Trace v9 writes whole paths, under this directory or others such as
/// `screencast/`.
const RESOURCES_PREFIX: &str = "resources/";

/// Normalize a blob reference to its path inside the archive: a v9 path
/// passes through, a v8 entry name gets its directory back.
fn to_resource_path(mut raw: String) -> String {
    if !raw.contains('/') {
        raw.insert_str(0, RESOURCES_PREFIX);
    }
    raw
}

/// `deserialize_with` for an optional blob reference; see [`to_resource_path`].
pub(crate) fn resource_path<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.map(to_resource_path))
}

/// `deserialize_with` for a required blob reference; see [`to_resource_path`].
pub(crate) fn required_resource_path<'de, D>(
    deserializer: D,
) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(to_resource_path(String::deserialize(deserializer)?))
}

/// Failure payload attached to an [`AfterEvent`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionError {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub message: String,
}

/// 2D coordinates for input events and click targets. Used in
/// [`InputEvent::point`] and [`AfterEvent::point`].
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
