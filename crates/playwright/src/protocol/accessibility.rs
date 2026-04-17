// Accessibility — accessibility tree snapshots
//
// See: https://playwright.dev/docs/api/class-accessibility

use crate::error::Result;
use crate::protocol::page::Page;
use serde_json::Value;

/// Options for `Accessibility::snapshot`.
///
/// See: <https://playwright.dev/docs/api/class-accessibility#accessibility-snapshot>
#[derive(Debug, Default, Clone)]
pub struct AccessibilitySnapshotOptions {
    /// Whether to prune uninteresting nodes from the tree.
    ///
    /// Defaults to `true`.
    pub interesting_only: Option<bool>,

    /// The root element for the snapshot.
    ///
    /// When not set, the snapshot is taken from the entire page.
    pub root: Option<crate::protocol::ElementHandle>,
}

/// Provides accessibility-tree inspection methods on a page.
///
/// Access via [`Page::accessibility`].
///
/// See: <https://playwright.dev/docs/api/class-accessibility>
#[derive(Clone)]
pub struct Accessibility {
    page: Page,
}

impl Accessibility {
    pub(crate) fn new(page: Page) -> Self {
        Self { page }
    }

    /// Captures the current state of the page's accessibility tree.
    ///
    /// Returns the accessibility tree as a JSON `Value` (tree of nodes with
    /// `role`, `name`, `value`, `children`, etc.), or `null` when there is no
    /// accessibility tree.
    ///
    /// # Errors
    ///
    /// Returns error if the RPC call fails or the browser has been closed.
    ///
    /// See: <https://playwright.dev/docs/api/class-accessibility#accessibility-snapshot>
    pub async fn snapshot(&self, options: Option<AccessibilitySnapshotOptions>) -> Result<Value> {
        self.page.accessibility_snapshot(options).await
    }
}
