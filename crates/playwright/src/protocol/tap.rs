// Tap options and related types
//
// Provides configuration for tap actions, matching Playwright's API.
// Tap is very similar to click but sends touch events instead of mouse events.

use crate::protocol::click::{KeyboardModifier, Position};

/// Tap options
///
/// Configuration options for tap actions (touch-screen taps).
///
/// Use the builder pattern to construct options:
///
/// # Example
///
/// ```ignore
/// use playwright_rs::TapOptions;
///
/// // Tap with force (bypass actionability checks)
/// let options = TapOptions::builder()
///     .force(true)
///     .build();
///
/// // Trial run (actionability checks only, don't actually tap)
/// let options = TapOptions::builder()
///     .trial(true)
///     .build();
/// ```
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-tap>
#[derive(Debug, Clone, Default)]
pub struct TapOptions {
    /// Whether to bypass actionability checks
    pub force: Option<bool>,
    /// Modifier keys to press during tap
    pub modifiers: Option<Vec<KeyboardModifier>>,
    /// Position to tap relative to element top-left corner
    pub position: Option<Position>,
    /// Maximum time in milliseconds
    pub timeout: Option<f64>,
    /// Perform actionability checks without tapping
    pub trial: Option<bool>,
}

impl TapOptions {
    /// Create a new builder for TapOptions
    pub fn builder() -> TapOptionsBuilder {
        TapOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(force) = self.force {
            json["force"] = serde_json::json!(force);
        }

        if let Some(modifiers) = &self.modifiers {
            json["modifiers"] =
                serde_json::to_value(modifiers).expect("serialization of modifiers cannot fail");
        }

        if let Some(position) = &self.position {
            json["position"] =
                serde_json::to_value(position).expect("serialization of position cannot fail");
        }

        // Timeout is required in Playwright 1.56.1+
        if let Some(timeout) = self.timeout {
            json["timeout"] = serde_json::json!(timeout);
        } else {
            json["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        if let Some(trial) = self.trial {
            json["trial"] = serde_json::json!(trial);
        }

        json
    }
}

/// Builder for TapOptions
///
/// Provides a fluent API for constructing tap options.
#[derive(Debug, Clone, Default)]
pub struct TapOptionsBuilder {
    force: Option<bool>,
    modifiers: Option<Vec<KeyboardModifier>>,
    position: Option<Position>,
    timeout: Option<f64>,
    trial: Option<bool>,
}

impl TapOptionsBuilder {
    /// Bypass actionability checks
    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    /// Set modifier keys to press during tap
    pub fn modifiers(mut self, modifiers: Vec<KeyboardModifier>) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// Set position to tap relative to element top-left corner
    pub fn position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Perform actionability checks without tapping
    pub fn trial(mut self, trial: bool) -> Self {
        self.trial = Some(trial);
        self
    }

    /// Build the TapOptions
    pub fn build(self) -> TapOptions {
        TapOptions {
            force: self.force,
            modifiers: self.modifiers,
            position: self.position,
            timeout: self.timeout,
            trial: self.trial,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tap_options_default() {
        let options = TapOptions::builder().build();
        let json = options.to_json();
        // timeout has a default value
        assert!(json["timeout"].is_number());
        // other fields are absent
        assert!(json.get("force").is_none());
        assert!(json.get("trial").is_none());
    }

    #[test]
    fn test_tap_options_force() {
        let options = TapOptions::builder().force(true).build();
        let json = options.to_json();
        assert_eq!(json["force"], true);
    }

    #[test]
    fn test_tap_options_timeout() {
        let options = TapOptions::builder().timeout(5000.0).build();
        let json = options.to_json();
        assert_eq!(json["timeout"], 5000.0);
    }

    #[test]
    fn test_tap_options_trial() {
        let options = TapOptions::builder().trial(true).build();
        let json = options.to_json();
        assert_eq!(json["trial"], true);
    }

    #[test]
    fn test_tap_options_position() {
        let options = TapOptions::builder()
            .position(Position { x: 10.0, y: 20.0 })
            .build();
        let json = options.to_json();
        assert_eq!(json["position"]["x"], 10.0);
        assert_eq!(json["position"]["y"], 20.0);
    }
}
