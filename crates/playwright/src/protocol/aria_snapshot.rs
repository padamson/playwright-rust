//! Options for [`Locator::aria_snapshot`](crate::protocol::Locator::aria_snapshot)
//! and [`Page::aria_snapshot`](crate::protocol::Page::aria_snapshot).

/// Snapshot rendering mode.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-aria-snapshot>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
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
    /// When `true`, append each element's bounding box as
    /// `[box=x,y,width,height]` (useful for AI/LLM visual reasoning).
    pub boxes: Option<bool>,
    /// Override the default timeout (milliseconds).
    pub timeout: Option<f64>,
}

impl AriaSnapshotOptions {
    /// Snapshot mode (e.g. AI-oriented output).
    pub fn mode(mut self, mode: AriaSnapshotMode) -> Self {
        self.mode = Some(mode);
        self
    }
    /// Tracking identifier echoed back in the snapshot.
    pub fn track(mut self, track: impl Into<String>) -> Self {
        self.track = Some(track.into());
        self
    }
    /// Limit the snapshot to the given tree depth.
    pub fn depth(mut self, depth: i32) -> Self {
        self.depth = Some(depth);
        self
    }
    /// Append each element's bounding box as `[box=x,y,width,height]`.
    pub fn boxes(mut self, boxes: bool) -> Self {
        self.boxes = Some(boxes);
        self
    }
    /// Maximum time in milliseconds.
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }
}
