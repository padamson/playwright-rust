// Touchscreen - Low-level touchscreen control
//
// See: https://playwright.dev/docs/api/class-touchscreen

use crate::error::Result;
use crate::protocol::page::Page;

/// Touchscreen provides touch input simulation via a single touch point.
///
/// Access via [`Page::touchscreen()`].
///
/// See: <https://playwright.dev/docs/api/class-touchscreen>
#[derive(Clone)]
pub struct Touchscreen {
    page: Page,
}

impl Touchscreen {
    /// Creates a new Touchscreen instance for the given page.
    pub(crate) fn new(page: Page) -> Self {
        Self { page }
    }

    /// Sends a single touch to the specified coordinates.
    ///
    /// Dispatches a `touchstart` and `touchend` event sequence at the given
    /// viewport coordinates. Requires a touch-enabled browser context
    /// (`has_touch: true` in [`BrowserContextOptions`](crate::protocol::BrowserContextOptions)).
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in CSS pixels (relative to viewport top-left)
    /// * `y` - Y coordinate in CSS pixels (relative to viewport top-left)
    ///
    /// # Errors
    ///
    /// Returns error if the RPC call fails or the page has been closed.
    ///
    /// See: <https://playwright.dev/docs/api/class-touchscreen#touchscreen-tap>
    pub async fn tap(&self, x: f64, y: f64) -> Result<()> {
        self.page.touchscreen_tap(x, y).await
    }
}
