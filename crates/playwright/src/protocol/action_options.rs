// Action options for various Locator methods
//
// Provides configuration for fill, press, check, hover, and select actions.

use super::click::{KeyboardModifier, Position};

/// Fill options
///
/// Configuration options for fill() action.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-fill>
#[derive(Debug, Clone, Default)]
pub struct FillOptions {
    /// Whether to bypass actionability checks
    pub force: Option<bool>,
    /// Maximum time in milliseconds
    pub timeout: Option<f64>,
}

impl FillOptions {
    /// Create a new builder for FillOptions
    pub fn builder() -> FillOptionsBuilder {
        FillOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(force) = self.force {
            json["force"] = serde_json::json!(force);
        }

        // Timeout is required in Playwright 1.56.1+
        if let Some(timeout) = self.timeout {
            json["timeout"] = serde_json::json!(timeout);
        } else {
            json["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        json
    }
}

/// Builder for FillOptions
#[derive(Debug, Clone, Default)]
pub struct FillOptionsBuilder {
    force: Option<bool>,
    timeout: Option<f64>,
}

impl FillOptionsBuilder {
    /// Bypass actionability checks
    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the FillOptions
    pub fn build(self) -> FillOptions {
        FillOptions {
            force: self.force,
            timeout: self.timeout,
        }
    }
}

/// Press options
///
/// Configuration options for press() action.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-press>
#[derive(Debug, Clone, Default)]
pub struct PressOptions {
    /// Time to wait between keydown and keyup in milliseconds
    pub delay: Option<f64>,
    /// Maximum time in milliseconds
    pub timeout: Option<f64>,
}

impl PressOptions {
    /// Create a new builder for PressOptions
    pub fn builder() -> PressOptionsBuilder {
        PressOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(delay) = self.delay {
            json["delay"] = serde_json::json!(delay);
        }

        // Timeout is required in Playwright 1.56.1+
        if let Some(timeout) = self.timeout {
            json["timeout"] = serde_json::json!(timeout);
        } else {
            json["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        json
    }
}

/// Builder for PressOptions
#[derive(Debug, Clone, Default)]
pub struct PressOptionsBuilder {
    delay: Option<f64>,
    timeout: Option<f64>,
}

impl PressOptionsBuilder {
    /// Set delay between keydown and keyup in milliseconds
    pub fn delay(mut self, delay: f64) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the PressOptions
    pub fn build(self) -> PressOptions {
        PressOptions {
            delay: self.delay,
            timeout: self.timeout,
        }
    }
}

/// Check options
///
/// Configuration options for check() and uncheck() actions.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-check>
#[derive(Debug, Clone, Default)]
pub struct CheckOptions {
    /// Whether to bypass actionability checks
    pub force: Option<bool>,
    /// Position to click relative to element top-left corner
    pub position: Option<Position>,
    /// Maximum time in milliseconds
    pub timeout: Option<f64>,
    /// Perform actionability checks without checking
    pub trial: Option<bool>,
}

impl CheckOptions {
    /// Create a new builder for CheckOptions
    pub fn builder() -> CheckOptionsBuilder {
        CheckOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(force) = self.force {
            json["force"] = serde_json::json!(force);
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

/// Builder for CheckOptions
#[derive(Debug, Clone, Default)]
pub struct CheckOptionsBuilder {
    force: Option<bool>,
    position: Option<Position>,
    timeout: Option<f64>,
    trial: Option<bool>,
}

impl CheckOptionsBuilder {
    /// Bypass actionability checks
    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
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

    /// Perform actionability checks without checking
    pub fn trial(mut self, trial: bool) -> Self {
        self.trial = Some(trial);
        self
    }

    /// Build the CheckOptions
    pub fn build(self) -> CheckOptions {
        CheckOptions {
            force: self.force,
            position: self.position,
            timeout: self.timeout,
            trial: self.trial,
        }
    }
}

/// Hover options
///
/// Configuration options for hover() action.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-hover>
#[derive(Debug, Clone, Default)]
pub struct HoverOptions {
    /// Whether to bypass actionability checks
    pub force: Option<bool>,
    /// Modifier keys to press during hover
    pub modifiers: Option<Vec<KeyboardModifier>>,
    /// Position to hover relative to element top-left corner
    pub position: Option<Position>,
    /// Maximum time in milliseconds
    pub timeout: Option<f64>,
    /// Perform actionability checks without hovering
    pub trial: Option<bool>,
}

impl HoverOptions {
    /// Create a new builder for HoverOptions
    pub fn builder() -> HoverOptionsBuilder {
        HoverOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(force) = self.force {
            json["force"] = serde_json::json!(force);
        }

        if let Some(modifiers) = &self.modifiers {
            json["modifiers"] = serde_json::to_value(modifiers).unwrap();
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

/// Builder for HoverOptions
#[derive(Debug, Clone, Default)]
pub struct HoverOptionsBuilder {
    force: Option<bool>,
    modifiers: Option<Vec<KeyboardModifier>>,
    position: Option<Position>,
    timeout: Option<f64>,
    trial: Option<bool>,
}

impl HoverOptionsBuilder {
    /// Bypass actionability checks
    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    /// Set modifier keys to press during hover
    pub fn modifiers(mut self, modifiers: Vec<KeyboardModifier>) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// Set position to hover relative to element top-left corner
    pub fn position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Perform actionability checks without hovering
    pub fn trial(mut self, trial: bool) -> Self {
        self.trial = Some(trial);
        self
    }

    /// Build the HoverOptions
    pub fn build(self) -> HoverOptions {
        HoverOptions {
            force: self.force,
            modifiers: self.modifiers,
            position: self.position,
            timeout: self.timeout,
            trial: self.trial,
        }
    }
}

/// Select options
///
/// Configuration options for select_option() action.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-select-option>
#[derive(Debug, Clone, Default)]
pub struct SelectOptions {
    /// Whether to bypass actionability checks
    pub force: Option<bool>,
    /// Maximum time in milliseconds
    pub timeout: Option<f64>,
}

impl SelectOptions {
    /// Create a new builder for SelectOptions
    pub fn builder() -> SelectOptionsBuilder {
        SelectOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(force) = self.force {
            json["force"] = serde_json::json!(force);
        }

        // Timeout is required in Playwright 1.56.1+
        if let Some(timeout) = self.timeout {
            json["timeout"] = serde_json::json!(timeout);
        } else {
            json["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        json
    }
}

/// Builder for SelectOptions
#[derive(Debug, Clone, Default)]
pub struct SelectOptionsBuilder {
    force: Option<bool>,
    timeout: Option<f64>,
}

impl SelectOptionsBuilder {
    /// Bypass actionability checks
    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the SelectOptions
    pub fn build(self) -> SelectOptions {
        SelectOptions {
            force: self.force,
            timeout: self.timeout,
        }
    }
}

/// Keyboard options
///
/// Configuration options for keyboard.press() and keyboard.type_text() methods.
///
/// See: <https://playwright.dev/docs/api/class-keyboard#keyboard-press>
#[derive(Debug, Clone, Default)]
pub struct KeyboardOptions {
    /// Time to wait between key presses in milliseconds
    pub delay: Option<f64>,
}

impl KeyboardOptions {
    /// Create a new builder for KeyboardOptions
    pub fn builder() -> KeyboardOptionsBuilder {
        KeyboardOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(delay) = self.delay {
            json["delay"] = serde_json::json!(delay);
        }

        json
    }
}

/// Builder for KeyboardOptions
#[derive(Debug, Clone, Default)]
pub struct KeyboardOptionsBuilder {
    delay: Option<f64>,
}

impl KeyboardOptionsBuilder {
    /// Set delay between key presses in milliseconds
    pub fn delay(mut self, delay: f64) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Build the KeyboardOptions
    pub fn build(self) -> KeyboardOptions {
        KeyboardOptions { delay: self.delay }
    }
}

/// Mouse options
///
/// Configuration options for mouse methods.
///
/// See: <https://playwright.dev/docs/api/class-mouse>
#[derive(Debug, Clone, Default)]
pub struct MouseOptions {
    /// Mouse button to use
    pub button: Option<super::click::MouseButton>,
    /// Number of clicks
    pub click_count: Option<u32>,
    /// Time to wait between mousedown and mouseup in milliseconds
    pub delay: Option<f64>,
    /// Number of intermediate mousemove events (for move operations)
    pub steps: Option<u32>,
}

impl MouseOptions {
    /// Create a new builder for MouseOptions
    pub fn builder() -> MouseOptionsBuilder {
        MouseOptionsBuilder::default()
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

        if let Some(steps) = self.steps {
            json["steps"] = serde_json::json!(steps);
        }

        json
    }
}

/// Builder for MouseOptions
#[derive(Debug, Clone, Default)]
pub struct MouseOptionsBuilder {
    button: Option<super::click::MouseButton>,
    click_count: Option<u32>,
    delay: Option<f64>,
    steps: Option<u32>,
}

impl MouseOptionsBuilder {
    /// Set the mouse button
    pub fn button(mut self, button: super::click::MouseButton) -> Self {
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

    /// Set number of intermediate mousemove events
    pub fn steps(mut self, steps: u32) -> Self {
        self.steps = Some(steps);
        self
    }

    /// Build the MouseOptions
    pub fn build(self) -> MouseOptions {
        MouseOptions {
            button: self.button,
            click_count: self.click_count,
            delay: self.delay,
            steps: self.steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::click::MouseButton;

    #[test]
    fn test_fill_options_builder() {
        let options = FillOptions::builder().force(true).timeout(5000.0).build();

        let json = options.to_json();
        assert_eq!(json["force"], true);
        assert_eq!(json["timeout"], 5000.0);
    }

    #[test]
    fn test_press_options_builder() {
        let options = PressOptions::builder().delay(100.0).timeout(3000.0).build();

        let json = options.to_json();
        assert_eq!(json["delay"], 100.0);
        assert_eq!(json["timeout"], 3000.0);
    }

    #[test]
    fn test_check_options_builder() {
        let options = CheckOptions::builder()
            .force(true)
            .position(Position { x: 5.0, y: 10.0 })
            .timeout(2000.0)
            .trial(true)
            .build();

        let json = options.to_json();
        assert_eq!(json["force"], true);
        assert_eq!(json["position"]["x"], 5.0);
        assert_eq!(json["position"]["y"], 10.0);
        assert_eq!(json["timeout"], 2000.0);
        assert_eq!(json["trial"], true);
    }

    #[test]
    fn test_hover_options_builder() {
        let options = HoverOptions::builder()
            .force(true)
            .modifiers(vec![KeyboardModifier::Shift])
            .position(Position { x: 10.0, y: 20.0 })
            .timeout(4000.0)
            .trial(false)
            .build();

        let json = options.to_json();
        assert_eq!(json["force"], true);
        assert_eq!(json["modifiers"], serde_json::json!(["Shift"]));
        assert_eq!(json["position"]["x"], 10.0);
        assert_eq!(json["position"]["y"], 20.0);
        assert_eq!(json["timeout"], 4000.0);
        assert_eq!(json["trial"], false);
    }

    #[test]
    fn test_select_options_builder() {
        let options = SelectOptions::builder().force(true).timeout(6000.0).build();

        let json = options.to_json();
        assert_eq!(json["force"], true);
        assert_eq!(json["timeout"], 6000.0);
    }

    #[test]
    fn test_keyboard_options_builder() {
        let options = KeyboardOptions::builder().delay(50.0).build();

        let json = options.to_json();
        assert_eq!(json["delay"], 50.0);
    }

    #[test]
    fn test_mouse_options_builder() {
        let options = MouseOptions::builder()
            .button(MouseButton::Right)
            .click_count(2)
            .delay(100.0)
            .steps(10)
            .build();

        let json = options.to_json();
        assert_eq!(json["button"], "right");
        assert_eq!(json["clickCount"], 2);
        assert_eq!(json["delay"], 100.0);
        assert_eq!(json["steps"], 10);
    }
}
