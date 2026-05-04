//! Action — reassembled from `before` + optional `input` + zero-or-more
//! `log` + `after` events sharing a `call_id`.
//!
//! [`ActionStream`] consumes a stream of [`TraceEvent`]s and yields
//! [`Action`]s in `after`-arrival order. Truncated actions (no matching
//! `after` event) are emitted at end-of-stream rather than discarded —
//! useful when diagnosing crashed-mid-action traces.

use crate::error::Result;
use crate::event::{ActionError, AfterEvent, BeforeEvent, InputEvent, LogEvent, Point, TraceEvent};
use serde_json::Value;
use std::collections::HashMap;

/// A logical action — `class.method` call as recorded in the trace.
#[derive(Debug, Clone)]
pub struct Action {
    pub call_id: String,
    pub parent_id: Option<String>,
    pub class: String,
    pub method: String,
    pub title: Option<String>,
    pub page_id: Option<String>,
    pub start_time: f64,
    /// `None` for actions whose matching `after` event never arrived
    /// (truncated trace).
    pub end_time: Option<f64>,
    pub params: Value,
    pub result: Option<Value>,
    pub error: Option<ActionError>,
    pub logs: Vec<LogLine>,
    pub input: Option<InputEvent>,
    pub before_snapshot: Option<String>,
    pub after_snapshot: Option<String>,
    pub point: Option<Point>,
}

/// One log line attached to an action via the `log` event.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub time: f64,
    pub message: String,
}

impl From<LogEvent> for LogLine {
    fn from(value: LogEvent) -> Self {
        Self {
            time: value.time,
            message: value.message,
        }
    }
}

/// Streaming reassembly of [`Action`]s from a [`TraceEvent`] iterator.
/// Use [`crate::TraceReader::actions`] to construct the typical case;
/// public here so callers can wrap their own custom event source.
pub struct ActionStream<I> {
    events: I,
    pending: HashMap<String, ActionBuilder>,
    /// Order of `call_id` insertion, used to drain truncated actions
    /// in a deterministic order at end-of-stream.
    pending_order: Vec<String>,
    upstream_done: bool,
}

impl<I> ActionStream<I>
where
    I: Iterator<Item = Result<TraceEvent>>,
{
    pub fn new(events: I) -> Self {
        Self {
            events,
            pending: HashMap::new(),
            pending_order: Vec::new(),
            upstream_done: false,
        }
    }
}

impl<I> Iterator for ActionStream<I>
where
    I: Iterator<Item = Result<TraceEvent>>,
{
    type Item = Result<Action>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.upstream_done {
                // Drain truncated actions one at a time.
                while let Some(call_id) = self.pending_order.pop() {
                    if let Some(builder) = self.pending.remove(&call_id) {
                        return Some(Ok(builder.finalize_truncated()));
                    }
                }
                return None;
            }

            let event = match self.events.next() {
                Some(Ok(e)) => e,
                Some(Err(e)) => return Some(Err(e)),
                None => {
                    self.upstream_done = true;
                    continue;
                }
            };

            match event {
                TraceEvent::Before(b) => {
                    let call_id = b.call_id.clone();
                    if !self.pending.contains_key(&call_id) {
                        self.pending_order.push(call_id.clone());
                    }
                    self.pending.insert(call_id, ActionBuilder::from_before(b));
                }
                TraceEvent::Input(i) => {
                    if let Some(builder) = self.pending.get_mut(&i.call_id) {
                        builder.input = Some(i);
                    }
                    // Orphan input (no matching `before`) is dropped
                    // silently — typical for traces truncated at the
                    // head.
                }
                TraceEvent::Log(l) => {
                    if let Some(builder) = self.pending.get_mut(&l.call_id) {
                        builder.logs.push(l.into());
                    }
                }
                TraceEvent::After(a) => {
                    if let Some(builder) = self.pending.remove(&a.call_id) {
                        // Maintain pending_order: lazy removal at drain
                        // time. The vector may carry stale entries for
                        // already-finalised actions; the drain loop
                        // skips them via the `pending.remove` check.
                        return Some(Ok(builder.finalize(a)));
                    }
                    // Orphan after — ignore.
                }
                _ => {
                    // ContextOptions, Console, Event, FrameSnapshot,
                    // ScreencastFrame, Unknown: not part of action
                    // reassembly. Slice 1 ignores; later slices may
                    // index snapshots / surface console messages on
                    // the action.
                }
            }
        }
    }
}

struct ActionBuilder {
    call_id: String,
    parent_id: Option<String>,
    class: String,
    method: String,
    title: Option<String>,
    page_id: Option<String>,
    start_time: f64,
    params: Value,
    before_snapshot: Option<String>,
    logs: Vec<LogLine>,
    input: Option<InputEvent>,
}

impl ActionBuilder {
    fn from_before(b: BeforeEvent) -> Self {
        Self {
            call_id: b.call_id,
            parent_id: b.parent_id,
            class: b.class,
            method: b.method,
            title: b.title,
            page_id: b.page_id,
            start_time: b.start_time,
            params: b.params,
            before_snapshot: b.before_snapshot,
            logs: Vec::new(),
            input: None,
        }
    }

    fn finalize(self, a: AfterEvent) -> Action {
        Action {
            call_id: self.call_id,
            parent_id: self.parent_id,
            class: self.class,
            method: self.method,
            title: self.title,
            page_id: self.page_id,
            start_time: self.start_time,
            end_time: Some(a.end_time),
            params: self.params,
            result: a.result,
            error: a.error,
            logs: self.logs,
            input: self.input,
            before_snapshot: self.before_snapshot,
            after_snapshot: a.after_snapshot,
            point: a.point,
        }
    }

    fn finalize_truncated(self) -> Action {
        Action {
            call_id: self.call_id,
            parent_id: self.parent_id,
            class: self.class,
            method: self.method,
            title: self.title,
            page_id: self.page_id,
            start_time: self.start_time,
            end_time: None,
            params: self.params,
            result: None,
            error: None,
            logs: self.logs,
            input: self.input,
            before_snapshot: self.before_snapshot,
            after_snapshot: None,
            point: None,
        }
    }
}
