// WaitForOptions and WaitForState types
//
// Provides configuration for wait_for actions, matching Playwright's API.

use serde::Serialize;

/// The state to wait for when using [`Locator::wait_for()`].
///
/// Matches Playwright's `WaitForSelectorState` across all language bindings.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-wait-for>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WaitForState {
    /// Wait for the element to be present in the DOM (attached).
    Attached,
    /// Wait for the element to be removed from the DOM.
    Detached,
    /// Wait for the element to be visible (the default).
    Visible,
    /// Wait for the element to be hidden (invisible or not in the DOM).
    Hidden,
}

/// Options for [`Locator::wait_for()`].
///
/// Configuration for waiting until an element satisfies a given state condition.
/// If no state is specified, defaults to `Visible`.
///
/// Use the builder pattern to construct options:
///
/// # Example
///
/// ```ignore
/// use playwright_rs::{WaitForOptions, WaitForState};
///
/// // Wait until the element is visible
/// let options = WaitForOptions::builder()
///     .state(WaitForState::Visible)
///     .build();
///
/// // Wait until the element is hidden, with a custom timeout
/// let options = WaitForOptions::builder()
///     .state(WaitForState::Hidden)
///     .timeout(5000.0)
///     .build();
/// ```
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-wait-for>
#[derive(Debug, Clone, Default)]
pub struct WaitForOptions {
    /// The element state to wait for (defaults to `Visible` if not set)
    pub state: Option<WaitForState>,
    /// Maximum time in milliseconds
    pub timeout: Option<f64>,
}

impl WaitForOptions {
    /// Create a new builder for WaitForOptions
    pub fn builder() -> WaitForOptionsBuilder {
        WaitForOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        // Default to "visible" when no state is specified (matches Playwright behavior)
        let state = self.state.unwrap_or(WaitForState::Visible);
        json["state"] = serde_json::to_value(state).unwrap();

        // Timeout is required in Playwright 1.56.1+
        if let Some(timeout) = self.timeout {
            json["timeout"] = serde_json::json!(timeout);
        } else {
            json["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        json
    }
}

/// Builder for WaitForOptions
///
/// Provides a fluent API for constructing wait_for options.
#[derive(Debug, Clone, Default)]
pub struct WaitForOptionsBuilder {
    state: Option<WaitForState>,
    timeout: Option<f64>,
}

impl WaitForOptionsBuilder {
    /// Set the element state to wait for
    pub fn state(mut self, state: WaitForState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the WaitForOptions
    pub fn build(self) -> WaitForOptions {
        WaitForOptions {
            state: self.state,
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_for_state_serialization() {
        assert_eq!(
            serde_json::to_string(&WaitForState::Attached).unwrap(),
            "\"attached\""
        );
        assert_eq!(
            serde_json::to_string(&WaitForState::Detached).unwrap(),
            "\"detached\""
        );
        assert_eq!(
            serde_json::to_string(&WaitForState::Visible).unwrap(),
            "\"visible\""
        );
        assert_eq!(
            serde_json::to_string(&WaitForState::Hidden).unwrap(),
            "\"hidden\""
        );
    }

    #[test]
    fn test_wait_for_options_default_state() {
        // When no state is set, to_json() should produce "visible"
        let options = WaitForOptions::builder().build();
        let json = options.to_json();
        assert_eq!(json["state"], "visible");
        assert!(json["timeout"].is_number());
    }

    #[test]
    fn test_wait_for_options_all_states() {
        for (state, expected) in &[
            (WaitForState::Attached, "attached"),
            (WaitForState::Detached, "detached"),
            (WaitForState::Visible, "visible"),
            (WaitForState::Hidden, "hidden"),
        ] {
            let options = WaitForOptions::builder().state(*state).build();
            let json = options.to_json();
            assert_eq!(json["state"], *expected);
        }
    }

    #[test]
    fn test_wait_for_options_timeout() {
        let options = WaitForOptions::builder().timeout(5000.0).build();
        let json = options.to_json();
        assert_eq!(json["timeout"], 5000.0);
    }
}
