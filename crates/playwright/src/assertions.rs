// Assertions - Auto-retry assertions for testing
//
// Provides expect() API with auto-retry logic matching Playwright's assertions.
//
// See: https://playwright.dev/docs/test-assertions

use crate::error::Result;
use crate::protocol::Locator;
use std::time::Duration;

/// Default timeout for assertions (5 seconds, matching Playwright)
const DEFAULT_ASSERTION_TIMEOUT: Duration = Duration::from_secs(5);

/// Default polling interval for assertions (100ms)
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Creates an expectation for a locator with auto-retry behavior.
///
/// Assertions will retry until they pass or timeout (default: 5 seconds).
///
/// # Example
///
/// ```ignore
/// use playwright_rs::{expect, protocol::Playwright};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let browser = playwright.chromium().launch().await?;
///     let page = browser.new_page().await?;
///
///     // Test to_be_visible and to_be_hidden
///     page.goto("data:text/html,<button id='btn'>Click me</button><div id='hidden' style='display:none'>Hidden</div>", None).await?;
///     expect(page.locator("#btn").await).to_be_visible().await?;
///     expect(page.locator("#hidden").await).to_be_hidden().await?;
///
///     // Test not() negation
///     expect(page.locator("#btn").await).not().to_be_hidden().await?;
///     expect(page.locator("#hidden").await).not().to_be_visible().await?;
///
///     // Test with_timeout()
///     page.goto("data:text/html,<div id='element'>Visible</div>", None).await?;
///     expect(page.locator("#element").await)
///         .with_timeout(Duration::from_secs(10))
///         .to_be_visible()
///         .await?;
///
///     // Test to_be_enabled and to_be_disabled
///     page.goto("data:text/html,<button id='enabled'>Enabled</button><button id='disabled' disabled>Disabled</button>", None).await?;
///     expect(page.locator("#enabled").await).to_be_enabled().await?;
///     expect(page.locator("#disabled").await).to_be_disabled().await?;
///
///     // Test to_be_checked and to_be_unchecked
///     page.goto("data:text/html,<input type='checkbox' id='checked' checked><input type='checkbox' id='unchecked'>", None).await?;
///     expect(page.locator("#checked").await).to_be_checked().await?;
///     expect(page.locator("#unchecked").await).to_be_unchecked().await?;
///
///     // Test to_be_editable
///     page.goto("data:text/html,<input type='text' id='editable'>", None).await?;
///     expect(page.locator("#editable").await).to_be_editable().await?;
///
///     // Test to_be_focused
///     page.goto("data:text/html,<input type='text' id='input'>", None).await?;
///     page.evaluate::<(), ()>("document.getElementById('input').focus()", None).await?;
///     expect(page.locator("#input").await).to_be_focused().await?;
///
///     // Test to_contain_text
///     page.goto("data:text/html,<div id='content'>Hello World</div>", None).await?;
///     expect(page.locator("#content").await).to_contain_text("Hello").await?;
///     expect(page.locator("#content").await).to_contain_text("World").await?;
///
///     // Test to_have_text
///     expect(page.locator("#content").await).to_have_text("Hello World").await?;
///
///     // Test to_have_value
///     page.goto("data:text/html,<input type='text' id='input' value='test value'>", None).await?;
///     expect(page.locator("#input").await).to_have_value("test value").await?;
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/test-assertions>
pub fn expect(locator: Locator) -> Expectation {
    Expectation::new(locator)
}

/// Expectation wraps a locator and provides assertion methods with auto-retry.
pub struct Expectation {
    locator: Locator,
    timeout: Duration,
    poll_interval: Duration,
    negate: bool,
}

// Allow clippy::wrong_self_convention for to_* methods that consume self
// This matches Playwright's expect API pattern where assertions are chained and consumed
#[allow(clippy::wrong_self_convention)]
impl Expectation {
    /// Creates a new expectation for the given locator.
    pub(crate) fn new(locator: Locator) -> Self {
        Self {
            locator,
            timeout: DEFAULT_ASSERTION_TIMEOUT,
            poll_interval: DEFAULT_POLL_INTERVAL,
            negate: false,
        }
    }

    /// Sets a custom timeout for this assertion.
    ///
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets a custom poll interval for this assertion.
    ///
    /// Default is 100ms.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Negates the assertion.
    ///
    /// Note: We intentionally use `.not()` method instead of implementing `std::ops::Not`
    /// to match Playwright's API across all language bindings (JS/Python/Java/.NET).
    #[allow(clippy::should_implement_trait)]
    pub fn not(mut self) -> Self {
        self.negate = true;
        self
    }

    /// Asserts that the element is visible.
    ///
    /// This assertion will retry until the element becomes visible or timeout.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-be-visible>
    pub async fn to_be_visible(self) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();

        loop {
            let is_visible = self.locator.is_visible().await?;

            // Check if condition matches (with negation support)
            let matches = if self.negate { !is_visible } else { is_visible };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected element '{}' NOT to be visible, but it was visible after {:?}",
                        selector, self.timeout
                    )
                } else {
                    format!(
                        "Expected element '{}' to be visible, but it was not visible after {:?}",
                        selector, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the element is hidden (not visible).
    ///
    /// This assertion will retry until the element becomes hidden or timeout.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-be-hidden>
    pub async fn to_be_hidden(self) -> Result<()> {
        // to_be_hidden is the opposite of to_be_visible
        // Use negation to reuse the visibility logic
        let negated = Expectation {
            negate: !self.negate, // Flip negation
            ..self
        };
        negated.to_be_visible().await
    }

    /// Asserts that the element has the specified text content (exact match).
    ///
    /// This assertion will retry until the element has the exact text or timeout.
    /// Text is trimmed before comparison.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-have-text>
    pub async fn to_have_text(self, expected: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();
        let expected = expected.trim();

        loop {
            // Get text content (using inner_text for consistency with Playwright)
            let actual_text = self.locator.inner_text().await?;
            let actual = actual_text.trim();

            // Check if condition matches (with negation support)
            let matches = if self.negate {
                actual != expected
            } else {
                actual == expected
            };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected element '{}' NOT to have text '{}', but it did after {:?}",
                        selector, expected, self.timeout
                    )
                } else {
                    format!(
                        "Expected element '{}' to have text '{}', but had '{}' after {:?}",
                        selector, expected, actual, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the element's text matches the specified regex pattern.
    ///
    /// This assertion will retry until the element's text matches the pattern or timeout.
    pub async fn to_have_text_regex(self, pattern: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();
        let re = regex::Regex::new(pattern)
            .map_err(|e| crate::error::Error::InvalidArgument(format!("Invalid regex: {}", e)))?;

        loop {
            let actual_text = self.locator.inner_text().await?;
            let actual = actual_text.trim();

            // Check if condition matches (with negation support)
            let matches = if self.negate {
                !re.is_match(actual)
            } else {
                re.is_match(actual)
            };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected element '{}' NOT to match pattern '{}', but it did after {:?}",
                        selector, pattern, self.timeout
                    )
                } else {
                    format!(
                        "Expected element '{}' to match pattern '{}', but had '{}' after {:?}",
                        selector, pattern, actual, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the element contains the specified text (substring match).
    ///
    /// This assertion will retry until the element contains the text or timeout.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-contain-text>
    pub async fn to_contain_text(self, expected: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();

        loop {
            let actual_text = self.locator.inner_text().await?;
            let actual = actual_text.trim();

            // Check if condition matches (with negation support)
            let matches = if self.negate {
                !actual.contains(expected)
            } else {
                actual.contains(expected)
            };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected element '{}' NOT to contain text '{}', but it did after {:?}",
                        selector, expected, self.timeout
                    )
                } else {
                    format!(
                        "Expected element '{}' to contain text '{}', but had '{}' after {:?}",
                        selector, expected, actual, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the element's text contains a substring matching the regex pattern.
    ///
    /// This assertion will retry until the element contains the pattern or timeout.
    pub async fn to_contain_text_regex(self, pattern: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();
        let re = regex::Regex::new(pattern)
            .map_err(|e| crate::error::Error::InvalidArgument(format!("Invalid regex: {}", e)))?;

        loop {
            let actual_text = self.locator.inner_text().await?;
            let actual = actual_text.trim();

            // Check if condition matches (with negation support)
            let matches = if self.negate {
                !re.is_match(actual)
            } else {
                re.is_match(actual)
            };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected element '{}' NOT to contain pattern '{}', but it did after {:?}",
                        selector, pattern, self.timeout
                    )
                } else {
                    format!(
                        "Expected element '{}' to contain pattern '{}', but had '{}' after {:?}",
                        selector, pattern, actual, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the input element has the specified value.
    ///
    /// This assertion will retry until the input has the exact value or timeout.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-have-value>
    pub async fn to_have_value(self, expected: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();

        loop {
            let actual = self.locator.input_value(None).await?;

            // Check if condition matches (with negation support)
            let matches = if self.negate {
                actual != expected
            } else {
                actual == expected
            };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected input '{}' NOT to have value '{}', but it did after {:?}",
                        selector, expected, self.timeout
                    )
                } else {
                    format!(
                        "Expected input '{}' to have value '{}', but had '{}' after {:?}",
                        selector, expected, actual, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the input element's value matches the specified regex pattern.
    ///
    /// This assertion will retry until the input value matches the pattern or timeout.
    pub async fn to_have_value_regex(self, pattern: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();
        let re = regex::Regex::new(pattern)
            .map_err(|e| crate::error::Error::InvalidArgument(format!("Invalid regex: {}", e)))?;

        loop {
            let actual = self.locator.input_value(None).await?;

            // Check if condition matches (with negation support)
            let matches = if self.negate {
                !re.is_match(&actual)
            } else {
                re.is_match(&actual)
            };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected input '{}' NOT to match pattern '{}', but it did after {:?}",
                        selector, pattern, self.timeout
                    )
                } else {
                    format!(
                        "Expected input '{}' to match pattern '{}', but had '{}' after {:?}",
                        selector, pattern, actual, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the element is enabled.
    ///
    /// This assertion will retry until the element is enabled or timeout.
    /// An element is enabled if it does not have the "disabled" attribute.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-be-enabled>
    pub async fn to_be_enabled(self) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();

        loop {
            let is_enabled = self.locator.is_enabled().await?;

            // Check if condition matches (with negation support)
            let matches = if self.negate { !is_enabled } else { is_enabled };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected element '{}' NOT to be enabled, but it was enabled after {:?}",
                        selector, self.timeout
                    )
                } else {
                    format!(
                        "Expected element '{}' to be enabled, but it was not enabled after {:?}",
                        selector, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the element is disabled.
    ///
    /// This assertion will retry until the element is disabled or timeout.
    /// An element is disabled if it has the "disabled" attribute.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-be-disabled>
    pub async fn to_be_disabled(self) -> Result<()> {
        // to_be_disabled is the opposite of to_be_enabled
        // Use negation to reuse the enabled logic
        let negated = Expectation {
            negate: !self.negate, // Flip negation
            ..self
        };
        negated.to_be_enabled().await
    }

    /// Asserts that the checkbox or radio button is checked.
    ///
    /// This assertion will retry until the element is checked or timeout.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-be-checked>
    pub async fn to_be_checked(self) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();

        loop {
            let is_checked = self.locator.is_checked().await?;

            // Check if condition matches (with negation support)
            let matches = if self.negate { !is_checked } else { is_checked };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected element '{}' NOT to be checked, but it was checked after {:?}",
                        selector, self.timeout
                    )
                } else {
                    format!(
                        "Expected element '{}' to be checked, but it was not checked after {:?}",
                        selector, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the checkbox or radio button is unchecked.
    ///
    /// This assertion will retry until the element is unchecked or timeout.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-be-checked>
    pub async fn to_be_unchecked(self) -> Result<()> {
        // to_be_unchecked is the opposite of to_be_checked
        // Use negation to reuse the checked logic
        let negated = Expectation {
            negate: !self.negate, // Flip negation
            ..self
        };
        negated.to_be_checked().await
    }

    /// Asserts that the element is editable.
    ///
    /// This assertion will retry until the element is editable or timeout.
    /// An element is editable if it is enabled and does not have the "readonly" attribute.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-be-editable>
    pub async fn to_be_editable(self) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();

        loop {
            let is_editable = self.locator.is_editable().await?;

            // Check if condition matches (with negation support)
            let matches = if self.negate {
                !is_editable
            } else {
                is_editable
            };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected element '{}' NOT to be editable, but it was editable after {:?}",
                        selector, self.timeout
                    )
                } else {
                    format!(
                        "Expected element '{}' to be editable, but it was not editable after {:?}",
                        selector, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the element is focused (currently has focus).
    ///
    /// This assertion will retry until the element becomes focused or timeout.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-be-focused>
    pub async fn to_be_focused(self) -> Result<()> {
        let start = std::time::Instant::now();
        let selector = self.locator.selector().to_string();

        loop {
            let is_focused = self.locator.is_focused().await?;

            // Check if condition matches (with negation support)
            let matches = if self.negate { !is_focused } else { is_focused };

            if matches {
                return Ok(());
            }

            // Check timeout
            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected element '{}' NOT to be focused, but it was focused after {:?}",
                        selector, self.timeout
                    )
                } else {
                    format!(
                        "Expected element '{}' to be focused, but it was not focused after {:?}",
                        selector, self.timeout
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            // Wait before next poll
            tokio::time::sleep(self.poll_interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expectation_defaults() {
        // Verify default timeout and poll interval constants
        assert_eq!(DEFAULT_ASSERTION_TIMEOUT, Duration::from_secs(5));
        assert_eq!(DEFAULT_POLL_INTERVAL, Duration::from_millis(100));
    }
}
