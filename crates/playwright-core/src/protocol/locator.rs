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
}

impl std::fmt::Debug for Locator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Locator")
            .field("selector", &self.selector)
            .finish()
    }
}
