// Assertions - Auto-retry assertions for testing
//
// Provides expect() API with auto-retry logic matching Playwright's assertions.
//
// See: https://playwright.dev/docs/test-assertions

use crate::error::Result;
use crate::protocol::{Locator, Page};
use std::path::Path;
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

    /// Asserts that a locator's screenshot matches a baseline image.
    ///
    /// On first run (no baseline file), saves the screenshot as the new baseline.
    /// On subsequent runs, compares the screenshot pixel-by-pixel against the baseline.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#locator-assertions-to-have-screenshot-1>
    pub async fn to_have_screenshot(
        self,
        baseline_path: impl AsRef<Path>,
        options: Option<ScreenshotAssertionOptions>,
    ) -> Result<()> {
        let opts = options.unwrap_or_default();
        let baseline_path = baseline_path.as_ref();

        // Disable animations if requested
        if opts.animations == Some(Animations::Disabled) {
            let _ = self
                .locator
                .evaluate_js(DISABLE_ANIMATIONS_JS, None::<&()>)
                .await;
        }

        // Build screenshot options with mask support
        let screenshot_opts = if let Some(ref mask_locators) = opts.mask {
            // Inject mask overlays before capturing
            let mask_js = build_mask_js(mask_locators);
            let _ = self.locator.evaluate_js(&mask_js, None::<&()>).await;
            None
        } else {
            None
        };

        compare_screenshot(
            &opts,
            baseline_path,
            self.timeout,
            self.poll_interval,
            self.negate,
            || async { self.locator.screenshot(screenshot_opts.clone()).await },
        )
        .await
    }
}

/// CSS to disable all animations and transitions
const DISABLE_ANIMATIONS_JS: &str = r#"
(() => {
    const style = document.createElement('style');
    style.textContent = '*, *::before, *::after { animation-duration: 0s !important; animation-delay: 0s !important; transition-duration: 0s !important; transition-delay: 0s !important; }';
    style.setAttribute('data-playwright-no-animations', '');
    document.head.appendChild(style);
})()
"#;

/// Build JavaScript to overlay mask regions with pink (#FF00FF) rectangles
fn build_mask_js(locators: &[Locator]) -> String {
    let selectors: Vec<String> = locators
        .iter()
        .map(|l| {
            let sel = l.selector().replace('\'', "\\'");
            format!(
                r#"
                (function() {{
                    var els = document.querySelectorAll('{}');
                    els.forEach(function(el) {{
                        var rect = el.getBoundingClientRect();
                        var overlay = document.createElement('div');
                        overlay.setAttribute('data-playwright-mask', '');
                        overlay.style.cssText = 'position:fixed;z-index:2147483647;background:#FF00FF;pointer-events:none;'
                            + 'left:' + rect.left + 'px;top:' + rect.top + 'px;width:' + rect.width + 'px;height:' + rect.height + 'px;';
                        document.body.appendChild(overlay);
                    }});
                }})();
                "#,
                sel
            )
        })
        .collect();
    selectors.join("\n")
}

/// Animation control for screenshots
///
/// See: <https://playwright.dev/docs/api/class-locatorassertions#locator-assertions-to-have-screenshot-1>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Animations {
    /// Allow animations to run normally
    Allow,
    /// Disable CSS animations and transitions before capturing
    Disabled,
}

/// Options for screenshot assertions
///
/// See: <https://playwright.dev/docs/api/class-locatorassertions#locator-assertions-to-have-screenshot-1>
#[derive(Debug, Clone, Default)]
pub struct ScreenshotAssertionOptions {
    /// Maximum number of different pixels allowed (default: 0)
    pub max_diff_pixels: Option<u32>,
    /// Maximum ratio of different pixels (0.0 to 1.0)
    pub max_diff_pixel_ratio: Option<f64>,
    /// Per-pixel color distance threshold (0.0 to 1.0, default: 0.2)
    pub threshold: Option<f64>,
    /// Disable CSS animations before capturing
    pub animations: Option<Animations>,
    /// Locators to mask with pink (#FF00FF) overlay
    pub mask: Option<Vec<Locator>>,
    /// Force update baseline even if it exists
    pub update_snapshots: Option<bool>,
}

impl ScreenshotAssertionOptions {
    /// Create a new builder for ScreenshotAssertionOptions
    pub fn builder() -> ScreenshotAssertionOptionsBuilder {
        ScreenshotAssertionOptionsBuilder::default()
    }
}

/// Builder for ScreenshotAssertionOptions
#[derive(Debug, Clone, Default)]
pub struct ScreenshotAssertionOptionsBuilder {
    max_diff_pixels: Option<u32>,
    max_diff_pixel_ratio: Option<f64>,
    threshold: Option<f64>,
    animations: Option<Animations>,
    mask: Option<Vec<Locator>>,
    update_snapshots: Option<bool>,
}

impl ScreenshotAssertionOptionsBuilder {
    /// Maximum number of different pixels allowed
    pub fn max_diff_pixels(mut self, pixels: u32) -> Self {
        self.max_diff_pixels = Some(pixels);
        self
    }

    /// Maximum ratio of different pixels (0.0 to 1.0)
    pub fn max_diff_pixel_ratio(mut self, ratio: f64) -> Self {
        self.max_diff_pixel_ratio = Some(ratio);
        self
    }

    /// Per-pixel color distance threshold (0.0 to 1.0)
    pub fn threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self
    }

    /// Disable CSS animations and transitions before capturing
    pub fn animations(mut self, animations: Animations) -> Self {
        self.animations = Some(animations);
        self
    }

    /// Locators to mask with pink (#FF00FF) overlay
    pub fn mask(mut self, locators: Vec<Locator>) -> Self {
        self.mask = Some(locators);
        self
    }

    /// Force update baseline even if it exists
    pub fn update_snapshots(mut self, update: bool) -> Self {
        self.update_snapshots = Some(update);
        self
    }

    /// Build the ScreenshotAssertionOptions
    pub fn build(self) -> ScreenshotAssertionOptions {
        ScreenshotAssertionOptions {
            max_diff_pixels: self.max_diff_pixels,
            max_diff_pixel_ratio: self.max_diff_pixel_ratio,
            threshold: self.threshold,
            animations: self.animations,
            mask: self.mask,
            update_snapshots: self.update_snapshots,
        }
    }
}

/// Creates a page-level expectation for screenshot assertions.
///
/// See: <https://playwright.dev/docs/test-assertions#page-assertions-to-have-screenshot-1>
pub fn expect_page(page: &Page) -> PageExpectation {
    PageExpectation::new(page.clone())
}

/// Page-level expectation for screenshot assertions.
#[allow(clippy::wrong_self_convention)]
pub struct PageExpectation {
    page: Page,
    timeout: Duration,
    poll_interval: Duration,
    negate: bool,
}

impl PageExpectation {
    fn new(page: Page) -> Self {
        Self {
            page,
            timeout: DEFAULT_ASSERTION_TIMEOUT,
            poll_interval: DEFAULT_POLL_INTERVAL,
            negate: false,
        }
    }

    /// Sets a custom timeout for this assertion.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Negates the assertion.
    #[allow(clippy::should_implement_trait)]
    pub fn not(mut self) -> Self {
        self.negate = true;
        self
    }

    /// Asserts that the page title matches the expected string.
    ///
    /// Auto-retries until the title matches or the timeout expires.
    ///
    /// See: <https://playwright.dev/docs/api/class-pageassertions#page-assertions-to-have-title>
    pub async fn to_have_title(self, expected: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let expected = expected.trim();

        loop {
            let actual = self.page.title().await?;
            let actual = actual.trim();

            let matches = if self.negate {
                actual != expected
            } else {
                actual == expected
            };

            if matches {
                return Ok(());
            }

            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected page NOT to have title '{}', but it did after {:?}",
                        expected, self.timeout,
                    )
                } else {
                    format!(
                        "Expected page to have title '{}', but got '{}' after {:?}",
                        expected, actual, self.timeout,
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the page title matches the given regex pattern.
    ///
    /// Auto-retries until the title matches or the timeout expires.
    ///
    /// See: <https://playwright.dev/docs/api/class-pageassertions#page-assertions-to-have-title>
    pub async fn to_have_title_regex(self, pattern: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let re = regex::Regex::new(pattern)
            .map_err(|e| crate::error::Error::InvalidArgument(format!("Invalid regex: {}", e)))?;

        loop {
            let actual = self.page.title().await?;

            let matches = if self.negate {
                !re.is_match(&actual)
            } else {
                re.is_match(&actual)
            };

            if matches {
                return Ok(());
            }

            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected page title NOT to match '{}', but '{}' matched after {:?}",
                        pattern, actual, self.timeout,
                    )
                } else {
                    format!(
                        "Expected page title to match '{}', but got '{}' after {:?}",
                        pattern, actual, self.timeout,
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the page URL matches the expected string.
    ///
    /// Auto-retries until the URL matches or the timeout expires.
    ///
    /// See: <https://playwright.dev/docs/api/class-pageassertions#page-assertions-to-have-url>
    pub async fn to_have_url(self, expected: &str) -> Result<()> {
        let start = std::time::Instant::now();

        loop {
            let actual = self.page.url();

            let matches = if self.negate {
                actual != expected
            } else {
                actual == expected
            };

            if matches {
                return Ok(());
            }

            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected page NOT to have URL '{}', but it did after {:?}",
                        expected, self.timeout,
                    )
                } else {
                    format!(
                        "Expected page to have URL '{}', but got '{}' after {:?}",
                        expected, actual, self.timeout,
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the page URL matches the given regex pattern.
    ///
    /// Auto-retries until the URL matches or the timeout expires.
    ///
    /// See: <https://playwright.dev/docs/api/class-pageassertions#page-assertions-to-have-url>
    pub async fn to_have_url_regex(self, pattern: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let re = regex::Regex::new(pattern)
            .map_err(|e| crate::error::Error::InvalidArgument(format!("Invalid regex: {}", e)))?;

        loop {
            let actual = self.page.url();

            let matches = if self.negate {
                !re.is_match(&actual)
            } else {
                re.is_match(&actual)
            };

            if matches {
                return Ok(());
            }

            if start.elapsed() >= self.timeout {
                let message = if self.negate {
                    format!(
                        "Expected page URL NOT to match '{}', but '{}' matched after {:?}",
                        pattern, actual, self.timeout,
                    )
                } else {
                    format!(
                        "Expected page URL to match '{}', but got '{}' after {:?}",
                        pattern, actual, self.timeout,
                    )
                };
                return Err(crate::error::Error::AssertionTimeout(message));
            }

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Asserts that the page screenshot matches a baseline image.
    ///
    /// See: <https://playwright.dev/docs/test-assertions#page-assertions-to-have-screenshot-1>
    pub async fn to_have_screenshot(
        self,
        baseline_path: impl AsRef<Path>,
        options: Option<ScreenshotAssertionOptions>,
    ) -> Result<()> {
        let opts = options.unwrap_or_default();
        let baseline_path = baseline_path.as_ref();

        // Disable animations if requested
        if opts.animations == Some(Animations::Disabled) {
            let _ = self.page.evaluate_expression(DISABLE_ANIMATIONS_JS).await;
        }

        // Inject mask overlays if specified
        if let Some(ref mask_locators) = opts.mask {
            let mask_js = build_mask_js(mask_locators);
            let _ = self.page.evaluate_expression(&mask_js).await;
        }

        compare_screenshot(
            &opts,
            baseline_path,
            self.timeout,
            self.poll_interval,
            self.negate,
            || async { self.page.screenshot(None).await },
        )
        .await
    }
}

/// Core screenshot comparison logic shared by Locator and Page assertions.
async fn compare_screenshot<F, Fut>(
    opts: &ScreenshotAssertionOptions,
    baseline_path: &Path,
    timeout: Duration,
    poll_interval: Duration,
    negate: bool,
    take_screenshot: F,
) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<u8>>>,
{
    let threshold = opts.threshold.unwrap_or(0.2);
    let max_diff_pixels = opts.max_diff_pixels;
    let max_diff_pixel_ratio = opts.max_diff_pixel_ratio;
    let update_snapshots = opts.update_snapshots.unwrap_or(false);

    // Take initial screenshot
    let actual_bytes = take_screenshot().await?;

    // If baseline doesn't exist or update_snapshots is set, save and return
    if !baseline_path.exists() || update_snapshots {
        if let Some(parent) = baseline_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                crate::error::Error::ProtocolError(format!(
                    "Failed to create baseline directory: {}",
                    e
                ))
            })?;
        }
        tokio::fs::write(baseline_path, &actual_bytes)
            .await
            .map_err(|e| {
                crate::error::Error::ProtocolError(format!(
                    "Failed to write baseline screenshot: {}",
                    e
                ))
            })?;
        return Ok(());
    }

    // Load baseline
    let baseline_bytes = tokio::fs::read(baseline_path).await.map_err(|e| {
        crate::error::Error::ProtocolError(format!("Failed to read baseline screenshot: {}", e))
    })?;

    let start = std::time::Instant::now();

    loop {
        let screenshot_bytes = if start.elapsed().is_zero() {
            actual_bytes.clone()
        } else {
            take_screenshot().await?
        };

        let comparison = compare_images(&baseline_bytes, &screenshot_bytes, threshold)?;

        let within_tolerance =
            is_within_tolerance(&comparison, max_diff_pixels, max_diff_pixel_ratio);

        let matches = if negate {
            !within_tolerance
        } else {
            within_tolerance
        };

        if matches {
            return Ok(());
        }

        if start.elapsed() >= timeout {
            if negate {
                return Err(crate::error::Error::AssertionTimeout(format!(
                    "Expected screenshots NOT to match, but they matched after {:?}",
                    timeout
                )));
            }

            // Save actual and diff images for debugging
            let baseline_stem = baseline_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("screenshot");
            let baseline_ext = baseline_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("png");
            let baseline_dir = baseline_path.parent().unwrap_or(Path::new("."));

            let actual_path =
                baseline_dir.join(format!("{}-actual.{}", baseline_stem, baseline_ext));
            let diff_path = baseline_dir.join(format!("{}-diff.{}", baseline_stem, baseline_ext));

            let _ = tokio::fs::write(&actual_path, &screenshot_bytes).await;

            if let Ok(diff_bytes) =
                generate_diff_image(&baseline_bytes, &screenshot_bytes, threshold)
            {
                let _ = tokio::fs::write(&diff_path, diff_bytes).await;
            }

            return Err(crate::error::Error::AssertionTimeout(format!(
                "Screenshot mismatch: {} pixels differ ({:.2}% of total). \
                 Max allowed: {}. Threshold: {:.2}. \
                 Actual saved to: {}. Diff saved to: {}. \
                 Timed out after {:?}",
                comparison.diff_count,
                comparison.diff_ratio * 100.0,
                max_diff_pixels
                    .map(|p| p.to_string())
                    .or_else(|| max_diff_pixel_ratio.map(|r| format!("{:.2}%", r * 100.0)))
                    .unwrap_or_else(|| "0".to_string()),
                threshold,
                actual_path.display(),
                diff_path.display(),
                timeout,
            )));
        }

        tokio::time::sleep(poll_interval).await;
    }
}

/// Result of comparing two images pixel-by-pixel
struct ImageComparison {
    diff_count: u32,
    diff_ratio: f64,
}

fn is_within_tolerance(
    comparison: &ImageComparison,
    max_diff_pixels: Option<u32>,
    max_diff_pixel_ratio: Option<f64>,
) -> bool {
    if let Some(max_pixels) = max_diff_pixels {
        if comparison.diff_count > max_pixels {
            return false;
        }
    } else if let Some(max_ratio) = max_diff_pixel_ratio {
        if comparison.diff_ratio > max_ratio {
            return false;
        }
    } else {
        // No tolerance specified — require exact match
        if comparison.diff_count > 0 {
            return false;
        }
    }
    true
}

/// Compare two PNG images pixel-by-pixel with a color distance threshold
fn compare_images(
    baseline_bytes: &[u8],
    actual_bytes: &[u8],
    threshold: f64,
) -> Result<ImageComparison> {
    use image::GenericImageView;

    let baseline_img = image::load_from_memory(baseline_bytes).map_err(|e| {
        crate::error::Error::ProtocolError(format!("Failed to decode baseline image: {}", e))
    })?;
    let actual_img = image::load_from_memory(actual_bytes).map_err(|e| {
        crate::error::Error::ProtocolError(format!("Failed to decode actual image: {}", e))
    })?;

    let (bw, bh) = baseline_img.dimensions();
    let (aw, ah) = actual_img.dimensions();

    // Different dimensions = all pixels differ
    if bw != aw || bh != ah {
        let total = bw.max(aw) * bh.max(ah);
        return Ok(ImageComparison {
            diff_count: total,
            diff_ratio: 1.0,
        });
    }

    let total_pixels = bw * bh;
    if total_pixels == 0 {
        return Ok(ImageComparison {
            diff_count: 0,
            diff_ratio: 0.0,
        });
    }

    let threshold_sq = threshold * threshold;
    let mut diff_count: u32 = 0;

    for y in 0..bh {
        for x in 0..bw {
            let bp = baseline_img.get_pixel(x, y);
            let ap = actual_img.get_pixel(x, y);

            // Compute normalized color distance (each channel 0.0-1.0)
            let dr = (bp[0] as f64 - ap[0] as f64) / 255.0;
            let dg = (bp[1] as f64 - ap[1] as f64) / 255.0;
            let db = (bp[2] as f64 - ap[2] as f64) / 255.0;
            let da = (bp[3] as f64 - ap[3] as f64) / 255.0;

            let dist_sq = (dr * dr + dg * dg + db * db + da * da) / 4.0;

            if dist_sq > threshold_sq {
                diff_count += 1;
            }
        }
    }

    Ok(ImageComparison {
        diff_count,
        diff_ratio: diff_count as f64 / total_pixels as f64,
    })
}

/// Generate a diff image highlighting differences in red
fn generate_diff_image(
    baseline_bytes: &[u8],
    actual_bytes: &[u8],
    threshold: f64,
) -> Result<Vec<u8>> {
    use image::{GenericImageView, ImageBuffer, Rgba};

    let baseline_img = image::load_from_memory(baseline_bytes).map_err(|e| {
        crate::error::Error::ProtocolError(format!("Failed to decode baseline image: {}", e))
    })?;
    let actual_img = image::load_from_memory(actual_bytes).map_err(|e| {
        crate::error::Error::ProtocolError(format!("Failed to decode actual image: {}", e))
    })?;

    let (bw, bh) = baseline_img.dimensions();
    let (aw, ah) = actual_img.dimensions();
    let width = bw.max(aw);
    let height = bh.max(ah);

    let threshold_sq = threshold * threshold;

    let mut diff_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);

    for y in 0..height {
        for x in 0..width {
            if x >= bw || y >= bh || x >= aw || y >= ah {
                // Out of bounds for one image — mark as diff
                diff_img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                continue;
            }

            let bp = baseline_img.get_pixel(x, y);
            let ap = actual_img.get_pixel(x, y);

            let dr = (bp[0] as f64 - ap[0] as f64) / 255.0;
            let dg = (bp[1] as f64 - ap[1] as f64) / 255.0;
            let db = (bp[2] as f64 - ap[2] as f64) / 255.0;
            let da = (bp[3] as f64 - ap[3] as f64) / 255.0;

            let dist_sq = (dr * dr + dg * dg + db * db + da * da) / 4.0;

            if dist_sq > threshold_sq {
                // Different — red highlight
                diff_img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            } else {
                // Same — semi-transparent grayscale of actual
                let gray = ((ap[0] as u16 + ap[1] as u16 + ap[2] as u16) / 3) as u8;
                diff_img.put_pixel(x, y, Rgba([gray, gray, gray, 100]));
            }
        }
    }

    let mut output = std::io::Cursor::new(Vec::new());
    diff_img
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to encode diff image: {}", e))
        })?;

    Ok(output.into_inner())
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
