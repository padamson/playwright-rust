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
/// # Examples
///
/// ```ignore
/// use playwright_rs::protocol::{Playwright, SelectOption};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let browser = playwright.chromium().launch().await?;
///     let page = browser.new_page().await?;
///
///     // Demonstrate set_checked() - checkbox interaction
///     let _ = page.goto(
///         "data:text/html,<input type='checkbox' id='cb'>",
///         None
///     ).await;
///     let checkbox = page.locator("#cb").await;
///     checkbox.set_checked(true, None).await?;
///     assert!(checkbox.is_checked().await?);
///     checkbox.set_checked(false, None).await?;
///     assert!(!checkbox.is_checked().await?);
///
///     // Demonstrate select_option() - select by value, label, and index
///     let _ = page.goto(
///         "data:text/html,<select id='fruits'>\
///             <option value='apple'>Apple</option>\
///             <option value='banana'>Banana</option>\
///             <option value='cherry'>Cherry</option>\
///         </select>",
///         None
///     ).await;
///     let select = page.locator("#fruits").await;
///     select.select_option("banana", None).await?;
///     assert_eq!(select.input_value(None).await?, "banana");
///     select.select_option(SelectOption::Label("Apple".to_string()), None).await?;
///     assert_eq!(select.input_value(None).await?, "apple");
///     select.select_option(SelectOption::Index(2), None).await?;
///     assert_eq!(select.input_value(None).await?, "cherry");
///
///     // Demonstrate select_option_multiple() - multi-select
///     let _ = page.goto(
///         "data:text/html,<select id='colors' multiple>\
///             <option value='red'>Red</option>\
///             <option value='green'>Green</option>\
///             <option value='blue'>Blue</option>\
///             <option value='yellow'>Yellow</option>\
///         </select>",
///         None
///     ).await;
///     let multi = page.locator("#colors").await;
///     let selected = multi.select_option_multiple(&["red", "blue"], None).await?;
///     assert_eq!(selected.len(), 2);
///     assert!(selected.contains(&"red".to_string()));
///     assert!(selected.contains(&"blue".to_string()));
///
///     // Demonstrate screenshot() - element screenshot
///     let _ = page.goto(
///         "data:text/html,<h1 id='title'>Hello World</h1>",
///         None
///     ).await;
///     let heading = page.locator("#title").await;
///     let screenshot = heading.screenshot(None).await?;
///     assert!(!screenshot.is_empty());
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
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

    /// Returns whether the element is focused (currently has focus).
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-focused>
    pub async fn is_focused(&self) -> Result<bool> {
        self.frame.locator_is_focused(&self.selector).await
    }

    // Action methods

    /// Clicks the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-click>
    pub async fn click(&self, options: Option<crate::protocol::ClickOptions>) -> Result<()> {
        self.frame
            .locator_click(&self.selector, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Wraps an error with selector context for better error messages.
    fn wrap_error_with_selector(&self, error: crate::error::Error) -> crate::error::Error {
        match &error {
            crate::error::Error::ProtocolError(msg) => {
                // Add selector context to protocol errors (timeouts, etc.)
                crate::error::Error::ProtocolError(format!("{} [selector: {}]", msg, self.selector))
            }
            crate::error::Error::Timeout(msg) => {
                crate::error::Error::Timeout(format!("{} [selector: {}]", msg, self.selector))
            }
            _ => error, // Other errors pass through unchanged
        }
    }

    /// Double clicks the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-dblclick>
    pub async fn dblclick(&self, options: Option<crate::protocol::ClickOptions>) -> Result<()> {
        self.frame.locator_dblclick(&self.selector, options).await
    }

    /// Fills the element with text.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-fill>
    pub async fn fill(
        &self,
        text: &str,
        options: Option<crate::protocol::FillOptions>,
    ) -> Result<()> {
        self.frame.locator_fill(&self.selector, text, options).await
    }

    /// Clears the element's value.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-clear>
    pub async fn clear(&self, options: Option<crate::protocol::FillOptions>) -> Result<()> {
        self.frame.locator_clear(&self.selector, options).await
    }

    /// Presses a key on the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-press>
    pub async fn press(
        &self,
        key: &str,
        options: Option<crate::protocol::PressOptions>,
    ) -> Result<()> {
        self.frame.locator_press(&self.selector, key, options).await
    }

    /// Ensures the checkbox or radio button is checked.
    ///
    /// This method is idempotent - if already checked, does nothing.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-check>
    pub async fn check(&self, options: Option<crate::protocol::CheckOptions>) -> Result<()> {
        self.frame.locator_check(&self.selector, options).await
    }

    /// Ensures the checkbox is unchecked.
    ///
    /// This method is idempotent - if already unchecked, does nothing.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-uncheck>
    pub async fn uncheck(&self, options: Option<crate::protocol::CheckOptions>) -> Result<()> {
        self.frame.locator_uncheck(&self.selector, options).await
    }

    /// Sets the checkbox or radio button to the specified checked state.
    ///
    /// This is a convenience method that calls `check()` if `checked` is true,
    /// or `uncheck()` if `checked` is false.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-checked>
    pub async fn set_checked(
        &self,
        checked: bool,
        options: Option<crate::protocol::CheckOptions>,
    ) -> Result<()> {
        if checked {
            self.check(options).await
        } else {
            self.uncheck(options).await
        }
    }

    /// Hovers the mouse over the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-hover>
    pub async fn hover(&self, options: Option<crate::protocol::HoverOptions>) -> Result<()> {
        self.frame.locator_hover(&self.selector, options).await
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
    pub async fn select_option(
        &self,
        value: impl Into<crate::protocol::SelectOption>,
        options: Option<crate::protocol::SelectOptions>,
    ) -> Result<Vec<String>> {
        self.frame
            .locator_select_option(&self.selector, value.into(), options)
            .await
    }

    /// Selects multiple options in a select element.
    ///
    /// Returns an array of option values that have been successfully selected.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-select-option>
    pub async fn select_option_multiple(
        &self,
        values: &[impl Into<crate::protocol::SelectOption> + Clone],
        options: Option<crate::protocol::SelectOptions>,
    ) -> Result<Vec<String>> {
        let select_options: Vec<crate::protocol::SelectOption> =
            values.iter().map(|v| v.clone().into()).collect();
        self.frame
            .locator_select_option_multiple(&self.selector, select_options, options)
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

    /// Sets a file to upload using FilePayload (explicit name, mimeType, buffer).
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
    pub async fn set_input_files_payload(
        &self,
        file: crate::protocol::FilePayload,
        _options: Option<()>,
    ) -> Result<()> {
        self.frame
            .locator_set_input_files_payload(&self.selector, file)
            .await
    }

    /// Sets multiple files to upload using FilePayload.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
    pub async fn set_input_files_payload_multiple(
        &self,
        files: &[crate::protocol::FilePayload],
        _options: Option<()>,
    ) -> Result<()> {
        self.frame
            .locator_set_input_files_payload_multiple(&self.selector, files)
            .await
    }

    /// Takes a screenshot of the element and returns the image bytes.
    ///
    /// This method uses strict mode - it will fail if the selector matches multiple elements.
    /// Use `first()`, `last()`, or `nth()` to refine the selector to a single element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-screenshot>
    pub async fn screenshot(
        &self,
        options: Option<crate::protocol::ScreenshotOptions>,
    ) -> Result<Vec<u8>> {
        // Query for the element using strict mode (should return exactly one)
        let element = self
            .frame
            .query_selector(&self.selector)
            .await?
            .ok_or_else(|| {
                crate::error::Error::ElementNotFound(format!(
                    "Element not found: {}",
                    self.selector
                ))
            })?;

        // Delegate to ElementHandle.screenshot()
        element.screenshot(options).await
    }
}

impl std::fmt::Debug for Locator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Locator")
            .field("selector", &self.selector)
            .finish()
    }
}
