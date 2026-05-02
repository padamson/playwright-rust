//! Options for [`Locator::aria_snapshot`](crate::protocol::Locator::aria_snapshot)
//! and [`Page::aria_snapshot`](crate::protocol::Page::aria_snapshot).

/// Snapshot rendering mode.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-aria-snapshot>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AriaSnapshotMode {
    /// AI-friendly form intended for LLM and codegen consumption — the
    /// snapshot is shaped to be easy to parse from inside a model
    /// prompt.
    Ai,
    /// Default human-readable form.
    Default,
}

impl AriaSnapshotMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            AriaSnapshotMode::Ai => "ai",
            AriaSnapshotMode::Default => "default",
        }
    }
}

/// Options accepted by `aria_snapshot()` on both Locator and Page.
#[derive(Debug, Clone, Default)]
pub struct AriaSnapshotOptions {
    /// Selects between human-readable (`Default`) and AI-friendly
    /// (`Ai`) snapshot output. Server default is `Default`.
    pub mode: Option<AriaSnapshotMode>,
    /// Track identifier — when supplied, the server may return an
    /// incremental snapshot relative to the previous request with the
    /// same track string.
    pub track: Option<String>,
    /// Maximum depth to descend in the accessibility tree.
    pub depth: Option<i32>,
    /// Override the default timeout (milliseconds).
    pub timeout: Option<f64>,
}
