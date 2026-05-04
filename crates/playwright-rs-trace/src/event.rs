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
        match serde_json::from_value::<TypedEnum>(Value::Object(self.raw.clone())) {
            Ok(t) => t.into(),
            Err(_) => TraceEvent::Unknown(self),
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
    FrameSnapshot(FrameSnapshotEvent),
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
            TypedEnum::FrameSnapshot(f) => TraceEvent::FrameSnapshot(f),
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
    #[serde(rename = "type", default)]
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

/// Per-frame DOM snapshot. Includes the full HTML payload — these can
/// be sizeable; callers iterating on snapshots for many frames should
/// expect the per-event size to dominate the overall trace memory
/// budget.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSnapshotEvent {
    pub call_id: String,
    pub snapshot_name: String,
    pub page_id: String,
    pub frame_id: String,
    #[serde(default)]
    pub frame_url: String,
    #[serde(default)]
    pub doctype: String,
    #[serde(default)]
    pub html: String,
    pub viewport: Option<Viewport>,
    pub timestamp: f64,
    #[serde(default)]
    pub wall_time: f64,
    #[serde(default)]
    pub collection_time: f64,
    #[serde(default)]
    pub is_main_frame: bool,
    #[serde(default)]
    pub resource_overrides: Vec<ResourceOverride>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

/// External resource reference used by a snapshot. Either a SHA-1 hash
/// (resolved through the zip's `resources/` directory) or an internal
/// reference identifier the trace viewer reassembles.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceOverride {
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(rename = "ref", default)]
    pub reference: Option<String>,
}

/// Single screencast frame stored as a JPEG in `resources/<sha1>`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastFrameEvent {
    pub page_id: String,
    pub sha1: String,
    pub width: u32,
    pub height: u32,
    pub timestamp: f64,
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
