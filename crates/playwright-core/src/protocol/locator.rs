// Locator - Lazy element selector with auto-waiting
//
// Locators are the central piece of Playwright's auto-waiting and retry-ability.
// They represent a way to find element(s) on the page at any given moment.
//
// Key characteristics:
// - Lazy: Don't execute until an action is performed
// - Retryable: Auto-wait for elements to match actionability checks
// - Chainable: Can create sub-locators via first(), last(), nth(), locator()
//
// Architecture:
// - Locator is NOT a ChannelOwner - it's a lightweight wrapper
// - Stores selector string and reference to Frame
// - Delegates all operations to Frame with strict=true
//
// See: https://playwright.dev/docs/api/class-locator

use crate::error::Result;
use crate::protocol::Frame;
use std::sync::Arc;

/// Locator represents a way to find element(s) on the page at any given moment.
///
/// Locators are lazy - they don't execute queries until an action is performed.
/// This enables auto-waiting and retry-ability for robust test automation.
///
/// See: <https://playwright.dev/docs/api/class-locator>
#[derive(Clone)]
pub struct Locator {
    frame: Arc<Frame>,
    selector: String,
}

impl Locator {
    /// Creates a new Locator (internal use only)
    ///
    /// Use `page.locator()` or `frame.locator()` to create locators in application code.
    pub(crate) fn new(frame: Arc<Frame>, selector: String) -> Self {
        Self { frame, selector }
    }

    /// Returns the selector string for this locator
    pub fn selector(&self) -> &str {
        &self.selector
    }

    /// Creates a locator for the first matching element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-first>
    pub fn first(&self) -> Locator {
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> nth=0", self.selector),
        )
    }

    /// Creates a locator for the last matching element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-last>
    pub fn last(&self) -> Locator {
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> nth=-1", self.selector),
        )
    }

    /// Creates a locator for the nth matching element (0-indexed).
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-nth>
    pub fn nth(&self, index: i32) -> Locator {
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> nth={}", self.selector, index),
        )
    }

    /// Creates a sub-locator within this locator's subtree.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-locator>
    pub fn locator(&self, selector: &str) -> Locator {
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> {}", self.selector, selector),
        )
    }

    /// Returns the number of elements matching this locator.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-count>
    pub async fn count(&self) -> Result<usize> {
        self.frame.locator_count(&self.selector).await
    }

    /// Returns the text content of the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-text-content>
    pub async fn text_content(&self) -> Result<Option<String>> {
        self.frame.locator_text_content(&self.selector).await
    }

    /// Returns the inner text of the element (visible text).
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-inner-text>
    pub async fn inner_text(&self) -> Result<String> {
        self.frame.locator_inner_text(&self.selector).await
    }

    /// Returns the inner HTML of the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-inner-html>
    pub async fn inner_html(&self) -> Result<String> {
        self.frame.locator_inner_html(&self.selector).await
    }

    /// Returns the value of the specified attribute.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-get-attribute>
    pub async fn get_attribute(&self, name: &str) -> Result<Option<String>> {
        self.frame.locator_get_attribute(&self.selector, name).await
    }

    /// Returns whether the element is visible.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-visible>
    pub async fn is_visible(&self) -> Result<bool> {
        self.frame.locator_is_visible(&self.selector).await
    }

    /// Returns whether the element is enabled.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-enabled>
    pub async fn is_enabled(&self) -> Result<bool> {
        self.frame.locator_is_enabled(&self.selector).await
    }

    /// Returns whether the checkbox or radio button is checked.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-checked>
    pub async fn is_checked(&self) -> Result<bool> {
        self.frame.locator_is_checked(&self.selector).await
    }

    /// Returns whether the element is editable.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-editable>
    pub async fn is_editable(&self) -> Result<bool> {
        self.frame.locator_is_editable(&self.selector).await
    }

    // Action methods

    /// Clicks the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-click>
    pub async fn click(&self, _options: Option<()>) -> Result<()> {
        self.frame.locator_click(&self.selector).await
    }

    /// Double clicks the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-dblclick>
    pub async fn dblclick(&self, _options: Option<()>) -> Result<()> {
        self.frame.locator_dblclick(&self.selector).await
    }

    /// Fills the element with text.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-fill>
    pub async fn fill(&self, text: &str, _options: Option<()>) -> Result<()> {
        self.frame.locator_fill(&self.selector, text).await
    }

    /// Clears the element's value.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-clear>
    pub async fn clear(&self, _options: Option<()>) -> Result<()> {
        self.frame.locator_clear(&self.selector).await
    }

    /// Presses a key on the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-press>
    pub async fn press(&self, key: &str, _options: Option<()>) -> Result<()> {
        self.frame.locator_press(&self.selector, key).await
    }

    /// Ensures the checkbox or radio button is checked.
    ///
    /// This method is idempotent - if already checked, does nothing.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-check>
    pub async fn check(&self, _options: Option<()>) -> Result<()> {
        self.frame.locator_check(&self.selector).await
    }

    /// Ensures the checkbox is unchecked.
    ///
    /// This method is idempotent - if already unchecked, does nothing.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-uncheck>
    pub async fn uncheck(&self, _options: Option<()>) -> Result<()> {
        self.frame.locator_uncheck(&self.selector).await
    }

    /// Hovers the mouse over the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-hover>
    pub async fn hover(&self, _options: Option<()>) -> Result<()> {
        self.frame.locator_hover(&self.selector).await
    }

    /// Returns the value of the input, textarea, or select element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-input-value>
    pub async fn input_value(&self, _options: Option<()>) -> Result<String> {
        self.frame.locator_input_value(&self.selector).await
    }

    /// Selects one or more options in a select element.
    ///
    /// Returns an array of option values that have been successfully selected.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-select-option>
    pub async fn select_option(&self, value: &str, _options: Option<()>) -> Result<Vec<String>> {
        self.frame
            .locator_select_option(&self.selector, value)
            .await
    }

    /// Selects multiple options in a select element.
    ///
    /// Returns an array of option values that have been successfully selected.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-select-option>
    pub async fn select_option_multiple(
        &self,
        values: &[&str],
        _options: Option<()>,
    ) -> Result<Vec<String>> {
        self.frame
            .locator_select_option_multiple(&self.selector, values)
            .await
    }

    /// Sets the file path(s) to upload to a file input element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
    pub async fn set_input_files(
        &self,
        file: &std::path::PathBuf,
        _options: Option<()>,
    ) -> Result<()> {
        self.frame
            .locator_set_input_files(&self.selector, file)
            .await
    }

    /// Sets multiple file paths to upload to a file input element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
    pub async fn set_input_files_multiple(
        &self,
        files: &[&std::path::PathBuf],
        _options: Option<()>,
    ) -> Result<()> {
        self.frame
            .locator_set_input_files_multiple(&self.selector, files)
            .await
    }

    // TODO: Element screenshots require ElementHandle protocol support
    // Deferred to Phase 4 - need to implement ElementHandles
    //
    // /// Takes a screenshot of the element and returns the image bytes.
    // ///
    // /// See: <https://playwright.dev/docs/api/class-locator#locator-screenshot>
    // pub async fn screenshot(&self, _options: Option<()>) -> Result<Vec<u8>> {
    //     self.frame.locator_screenshot(&self.selector).await
    // }
}

impl std::fmt::Debug for Locator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Locator")
            .field("selector", &self.selector)
            .finish()
    }
}
