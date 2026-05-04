//! Programmatic parser for [Playwright][pw] trace zip files
//! (format v8, matching Playwright 1.59.x).
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
mod trace;

pub use action::{Action, ActionStream, LogLine};
pub use error::{Result, TraceError};
pub use event::{
    ActionError, AfterEvent, BeforeEvent, ConsoleEvent, ConsoleLocation, ContextOptions,
    FrameSnapshotEvent, InputEvent, LogEvent, Point, RawEvent, ResourceOverride,
    ScreencastFrameEvent, SystemEvent, TraceEvent, Viewport,
};
pub use trace::{TraceReader, open};
