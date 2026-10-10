// Accessibility — accessibility tree snapshots
//
// See: https://playwright.dev/docs/api/class-accessibility

use crate::error::Result;
use crate::protocol::page::Page;
use serde_json::Value;

/// Options for `Accessibility::snapshot`.
///
/// See: <https://playwright.dev/docs/api/class-accessibility#accessibility-snapshot>
///
/// Both fields are accepted for parity with the other bindings and are
/// currently ignored: the driver this crate bundles has no accessibility
/// snapshot command, so [`Accessibility::snapshot`] is emulated with a
/// whole-page ARIA snapshot. Use [`Locator::aria_snapshot`] to scope a
/// snapshot to an element.
///
/// [`Locator::aria_snapshot`]: crate::protocol::Locator::aria_snapshot
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
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

impl AccessibilitySnapshotOptions {
    /// Whether to prune uninteresting nodes from the tree. Defaults to `true`.
    /// Currently ignored; see the struct docs.
    pub fn interesting_only(mut self, interesting_only: bool) -> Self {
        self.interesting_only = Some(interesting_only);
        self
    }

    /// The root element for the snapshot. Defaults to the entire page.
    /// Currently ignored; see the struct docs.
    pub fn root(mut self, root: crate::protocol::ElementHandle) -> Self {
        self.root = Some(root);
        self
    }
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
    pub async fn snapshot(
        &self,
        options: impl Into<Option<AccessibilitySnapshotOptions>>,
    ) -> Result<Value> {
        let options = options.into();
        self.page.accessibility_snapshot(options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_setter_writes_the_field() {
        let opts = AccessibilitySnapshotOptions::default().interesting_only(false);
        assert_eq!(opts.interesting_only, Some(false));
        assert!(opts.root.is_none());
    }
}
