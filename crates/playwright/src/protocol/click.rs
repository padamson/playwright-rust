// Click options and related types
//
// Provides configuration for click and dblclick actions, matching Playwright's API.

use serde::Serialize;

/// Mouse button for click actions
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::click::MouseButton;
///
/// let button = MouseButton::Right;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    /// Left mouse button (default)
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button
    Middle,
}

/// Keyboard modifier keys
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::click::KeyboardModifier;
///
/// let modifiers = vec![KeyboardModifier::Shift, KeyboardModifier::Control];
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum KeyboardModifier {
    /// Alt key
    Alt,
    /// Control key
    Control,
    /// Meta key (Command on macOS, Windows key on Windows)
    Meta,
    /// Shift key
    Shift,
    /// Control on Windows/Linux, Meta on macOS
    ControlOrMeta,
}

/// Position for click actions
///
/// Coordinates are relative to the top-left corner of the element's padding box.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::click::Position;
///
/// let position = Position { x: 10.0, y: 20.0 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Position {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

/// Click options
///
/// Configuration options for click and dblclick actions.
///
/// Use the builder pattern to construct options:
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::click::{ClickOptions, MouseButton, KeyboardModifier, Position};
///
/// // Right-click with modifiers
/// let options = ClickOptions::builder()
///     .button(MouseButton::Right)
///     .modifiers(vec![KeyboardModifier::Shift])
///     .build();
///
/// // Click at specific position
/// let options = ClickOptions::builder()
///     .position(Position { x: 10.0, y: 20.0 })
///     .build();
///
/// // Trial run (actionability checks only)
/// let options = ClickOptions::builder()
///     .trial(true)
///     .build();
/// ```
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-click>
#[derive(Debug, Clone, Default)]
pub struct ClickOptions {
    /// Mouse button to click (left, right, middle)
    pub button: Option<MouseButton>,
    /// Number of clicks (for multi-click)
    pub click_count: Option<u32>,
    /// Time to wait between mousedown and mouseup in milliseconds
    pub delay: Option<f64>,
    /// Whether to bypass actionability checks
    pub force: Option<bool>,
    /// Modifier keys to press during click
    pub modifiers: Option<Vec<KeyboardModifier>>,
    /// Don't wait for navigation after click
    pub no_wait_after: Option<bool>,
    /// Position to click relative to element top-left corner
    pub position: Option<Position>,
    /// Maximum time in milliseconds
    pub timeout: Option<f64>,
    /// Perform actionability checks without clicking
    pub trial: Option<bool>,
}

impl ClickOptions {
    /// Create a new builder for ClickOptions
    pub fn builder() -> ClickOptionsBuilder {
        ClickOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(button) = &self.button {
            json["button"] = serde_json::to_value(button).unwrap();
        }

        if let Some(click_count) = self.click_count {
            json["clickCount"] = serde_json::json!(click_count);
        }

        if let Some(delay) = self.delay {
            json["delay"] = serde_json::json!(delay);
        }

        if let Some(force) = self.force {
            json["force"] = serde_json::json!(force);
        }

        if let Some(modifiers) = &self.modifiers {
            json["modifiers"] = serde_json::to_value(modifiers).unwrap();
        }

        if let Some(no_wait_after) = self.no_wait_after {
            json["noWaitAfter"] = serde_json::json!(no_wait_after);
        }

        if let Some(position) = &self.position {
            json["position"] = serde_json::to_value(position).unwrap();
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

/// Builder for ClickOptions
///
/// Provides a fluent API for constructing click options.
#[derive(Debug, Clone, Default)]
pub struct ClickOptionsBuilder {
    button: Option<MouseButton>,
    click_count: Option<u32>,
    delay: Option<f64>,
    force: Option<bool>,
    modifiers: Option<Vec<KeyboardModifier>>,
    no_wait_after: Option<bool>,
    position: Option<Position>,
    timeout: Option<f64>,
    trial: Option<bool>,
}

impl ClickOptionsBuilder {
    /// Set the mouse button to click
    pub fn button(mut self, button: MouseButton) -> Self {
        self.button = Some(button);
        self
    }

    /// Set the number of clicks
    pub fn click_count(mut self, click_count: u32) -> Self {
        self.click_count = Some(click_count);
        self
    }

    /// Set delay between mousedown and mouseup in milliseconds
    pub fn delay(mut self, delay: f64) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Bypass actionability checks
    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    /// Set modifier keys to press during click
    pub fn modifiers(mut self, modifiers: Vec<KeyboardModifier>) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// Don't wait for navigation after click
    pub fn no_wait_after(mut self, no_wait_after: bool) -> Self {
        self.no_wait_after = Some(no_wait_after);
        self
    }

    /// Set position to click relative to element top-left corner
    pub fn position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Perform actionability checks without clicking
    pub fn trial(mut self, trial: bool) -> Self {
        self.trial = Some(trial);
        self
    }

    /// Build the ClickOptions
    pub fn build(self) -> ClickOptions {
        ClickOptions {
            button: self.button,
            click_count: self.click_count,
            delay: self.delay,
            force: self.force,
            modifiers: self.modifiers,
            no_wait_after: self.no_wait_after,
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
    fn test_mouse_button_serialization() {
        assert_eq!(
            serde_json::to_string(&MouseButton::Left).unwrap(),
            "\"left\""
        );
        assert_eq!(
            serde_json::to_string(&MouseButton::Right).unwrap(),
            "\"right\""
        );
        assert_eq!(
            serde_json::to_string(&MouseButton::Middle).unwrap(),
            "\"middle\""
        );
    }

    #[test]
    fn test_keyboard_modifier_serialization() {
        assert_eq!(
            serde_json::to_string(&KeyboardModifier::Alt).unwrap(),
            "\"Alt\""
        );
        assert_eq!(
            serde_json::to_string(&KeyboardModifier::Control).unwrap(),
            "\"Control\""
        );
        assert_eq!(
            serde_json::to_string(&KeyboardModifier::Meta).unwrap(),
            "\"Meta\""
        );
        assert_eq!(
            serde_json::to_string(&KeyboardModifier::Shift).unwrap(),
            "\"Shift\""
        );
        assert_eq!(
            serde_json::to_string(&KeyboardModifier::ControlOrMeta).unwrap(),
            "\"ControlOrMeta\""
        );
    }

    #[test]
    fn test_builder_button() {
        let options = ClickOptions::builder().button(MouseButton::Right).build();

        let json = options.to_json();
        assert_eq!(json["button"], "right");
    }

    #[test]
    fn test_builder_click_count() {
        let options = ClickOptions::builder().click_count(2).build();

        let json = options.to_json();
        assert_eq!(json["clickCount"], 2);
    }

    #[test]
    fn test_builder_delay() {
        let options = ClickOptions::builder().delay(100.0).build();

        let json = options.to_json();
        assert_eq!(json["delay"], 100.0);
    }

    #[test]
    fn test_builder_force() {
        let options = ClickOptions::builder().force(true).build();

        let json = options.to_json();
        assert_eq!(json["force"], true);
    }

    #[test]
    fn test_builder_modifiers() {
        let options = ClickOptions::builder()
            .modifiers(vec![KeyboardModifier::Shift, KeyboardModifier::Control])
            .build();

        let json = options.to_json();
        assert_eq!(json["modifiers"], serde_json::json!(["Shift", "Control"]));
    }

    #[test]
    fn test_builder_position() {
        let position = Position { x: 10.0, y: 20.0 };
        let options = ClickOptions::builder().position(position).build();

        let json = options.to_json();
        assert_eq!(json["position"]["x"], 10.0);
        assert_eq!(json["position"]["y"], 20.0);
    }

    #[test]
    fn test_builder_timeout() {
        let options = ClickOptions::builder().timeout(5000.0).build();

        let json = options.to_json();
        assert_eq!(json["timeout"], 5000.0);
    }

    #[test]
    fn test_builder_trial() {
        let options = ClickOptions::builder().trial(true).build();

        let json = options.to_json();
        assert_eq!(json["trial"], true);
    }

    #[test]
    fn test_builder_multiple_options() {
        let options = ClickOptions::builder()
            .button(MouseButton::Right)
            .modifiers(vec![KeyboardModifier::Shift])
            .position(Position { x: 5.0, y: 10.0 })
            .force(true)
            .timeout(3000.0)
            .build();

        let json = options.to_json();
        assert_eq!(json["button"], "right");
        assert_eq!(json["modifiers"], serde_json::json!(["Shift"]));
        assert_eq!(json["position"]["x"], 5.0);
        assert_eq!(json["position"]["y"], 10.0);
        assert_eq!(json["force"], true);
        assert_eq!(json["timeout"], 3000.0);
    }
}
