//! Programmatic parser for [Playwright][pw] trace zip files
//! (format v8, matching Playwright 1.59.x).
//!
//! # When to reach for this crate
//!
//! Pairs with the producer side, `playwright-rs::Tracing` (which
//! writes `.trace.zip` files during a test run). This crate is the
//! consumer side: a streaming, no-Playwright-server-required parser
//! for those files. Typical users:
//!
//! - CI bots that comment on PRs with "test X failed at this Locator"
//! - Dashboards that aggregate flaky-test root causes across runs
//! - AI agent feedback loops that learn from past trace failures
//! - Post-mortem analyzers run from a Rust binary or `xtask`
//!
//! No runtime dependency on the main `playwright-rs` crate — pull in
//! only this crate (typically as a `[dev-dependencies]` entry) when
//! you want to read traces.
//!
//! # Quick example
//!
//! ```rust,ignore
//! use playwright_rs_trace::{open, TraceEvent};
//!
//! let mut reader = open("trace.zip")?;
//! println!(
//!     "trace v{} from {}",
//!     reader.context().version,
//!     reader.context().browser_name,
//! );
//!
//! for action in reader.actions()? {
//!     let action = action?;
//!     if action.error.is_some() {
//!         eprintln!(
//!             "failed: {}.{} ({:?})",
//!             action.class, action.method, action.error,
//!         );
//!     }
//! }
//! # Ok::<(), playwright_rs_trace::TraceError>(())
//! ```
//!
//! The reader is a **streaming iterator** — events / actions are yielded
//! lazily as the underlying zip stream is read, so a large trace
//! doesn't need to fit in memory before processing begins.
//!
//! # Four streaming entry points on [`TraceReader`]
//!
//! - [`raw_events`] — every JSONL line as raw JSON. Forward-compat
//!   escape hatch for callers dispatching on event kinds we don't
//!   model.
//! - [`events`] — same lines parsed into a typed [`TraceEvent`] enum.
//!   Unknown / future kinds surface as [`TraceEvent::Unknown`].
//! - [`actions`] — `before` + optional `input` + zero-or-more `log` +
//!   `after` chunks reassembled into a logical [`Action`]. The common
//!   case; use this unless you specifically need the raw event stream.
//! - [`network`] — `NetworkEntry`s from the `trace.network` HAR-shape
//!   stream (request / response pairs). Independent of the action
//!   stream — collect-and-sort if you need a merged chronological
//!   view.
//!
//! [`raw_events`]: TraceReader::raw_events
//! [`events`]: TraceReader::events
//! [`actions`]: TraceReader::actions
//! [`network`]: TraceReader::network
//!
//! # Forward compatibility
//!
//! Every JSONL line is preserved losslessly via
//! [`TraceReader::raw_events`]. The typed iterators
//! ([`TraceReader::events`], [`TraceReader::actions`]) deserialize what
//! the parser models and route anything else to
//! [`TraceEvent::Unknown`] so nothing is silently dropped.
//!
//! See the crate `README.md` for the full slice-plan and roadmap.
//!
//! [pw]: https://playwright.dev/

mod action;
mod error;
mod event;
mod jsonl;
mod network;
mod trace;

pub use action::{Action, ActionStream, LogLine};
pub use error::{Result, TraceError};
pub use event::{
    ActionError, AfterEvent, BeforeEvent, ConsoleEvent, ConsoleLocation, ContextOptions,
    FrameSnapshotEvent, InputEvent, LogEvent, Point, RawEvent, ResourceOverride,
    ScreencastFrameEvent, SystemEvent, TraceEvent, Viewport,
};
pub use network::{
    HeaderEntry, NetworkEntry, RequestPostData, RequestSnapshot, ResponseContent, ResponseSnapshot,
};
pub use trace::{TraceReader, open};
