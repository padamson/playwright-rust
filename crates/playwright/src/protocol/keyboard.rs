// Keyboard - Low-level keyboard control
//
// See: https://playwright.dev/docs/api/class-keyboard

use crate::error::Result;
use crate::protocol::page::Page;

/// Keyboard provides low-level keyboard control.
///
/// See: <https://playwright.dev/docs/api/class-keyboard>
#[derive(Clone)]
pub struct Keyboard {
    page: Page,
}

impl Keyboard {
    /// Creates a new Keyboard instance for the given page
    pub(crate) fn new(page: Page) -> Self {
        Self { page }
    }

    /// Dispatches a `keydown` event.
    ///
    /// See: <https://playwright.dev/docs/api/class-keyboard#keyboard-down>
    pub async fn down(&self, key: &str) -> Result<()> {
        self.page.keyboard_down(key).await
    }

    /// Dispatches a `keyup` event.
    ///
    /// See: <https://playwright.dev/docs/api/class-keyboard#keyboard-up>
    pub async fn up(&self, key: &str) -> Result<()> {
        self.page.keyboard_up(key).await
    }

    /// Executes a complete key press (down + up sequence).
    ///
    /// See: <https://playwright.dev/docs/api/class-keyboard#keyboard-press>
    pub async fn press(
        &self,
        key: &str,
        options: Option<crate::protocol::KeyboardOptions>,
    ) -> Result<()> {
        self.page.keyboard_press(key, options).await
    }

    /// Sends a `keydown`, `keypress`/`input`, and `keyup` event for each character.
    ///
    /// See: <https://playwright.dev/docs/api/class-keyboard#keyboard-type>
    pub async fn type_text(
        &self,
        text: &str,
        options: Option<crate::protocol::KeyboardOptions>,
    ) -> Result<()> {
        self.page.keyboard_type(text, options).await
    }

    /// Dispatches only `input` event, does not emit `keydown`, `keyup` or `keypress` events.
    ///
    /// See: <https://playwright.dev/docs/api/class-keyboard#keyboard-insert-text>
    pub async fn insert_text(&self, text: &str) -> Result<()> {
        self.page.keyboard_insert_text(text).await
    }
}
