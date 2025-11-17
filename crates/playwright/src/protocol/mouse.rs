// Mouse - Low-level mouse control
//
// See: https://playwright.dev/docs/api/class-mouse

use crate::error::Result;
use crate::protocol::page::Page;

/// Mouse provides low-level mouse control.
///
/// Coordinates are in main-frame CSS pixels relative to the viewport's top-left corner.
///
/// See: <https://playwright.dev/docs/api/class-mouse>
#[derive(Clone)]
pub struct Mouse {
    page: Page,
}

impl Mouse {
    /// Creates a new Mouse instance for the given page
    pub(crate) fn new(page: Page) -> Self {
        Self { page }
    }

    /// Dispatches a `mousemove` event.
    ///
    /// See: <https://playwright.dev/docs/api/class-mouse#mouse-move>
    pub async fn move_to(
        &self,
        x: i32,
        y: i32,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        self.page.mouse_move(x, y, options).await
    }

    /// Combines `move()`, `down()`, and `up()` actions.
    ///
    /// See: <https://playwright.dev/docs/api/class-mouse#mouse-click>
    pub async fn click(
        &self,
        x: i32,
        y: i32,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        self.page.mouse_click(x, y, options).await
    }

    /// Shortcut performing `move()`, `down()`, `up()`, `down()`, and `up()` sequentially.
    ///
    /// See: <https://playwright.dev/docs/api/class-mouse#mouse-dblclick>
    pub async fn dblclick(
        &self,
        x: i32,
        y: i32,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        self.page.mouse_dblclick(x, y, options).await
    }

    /// Dispatches a `mousedown` event.
    ///
    /// See: <https://playwright.dev/docs/api/class-mouse#mouse-down>
    pub async fn down(&self, options: Option<crate::protocol::MouseOptions>) -> Result<()> {
        self.page.mouse_down(options).await
    }

    /// Dispatches a `mouseup` event.
    ///
    /// See: <https://playwright.dev/docs/api/class-mouse#mouse-up>
    pub async fn up(&self, options: Option<crate::protocol::MouseOptions>) -> Result<()> {
        self.page.mouse_up(options).await
    }

    /// Dispatches a `wheel` event for manual page scrolling.
    ///
    /// See: <https://playwright.dev/docs/api/class-mouse#mouse-wheel>
    pub async fn wheel(&self, delta_x: i32, delta_y: i32) -> Result<()> {
        self.page.mouse_wheel(delta_x, delta_y).await
    }
}
