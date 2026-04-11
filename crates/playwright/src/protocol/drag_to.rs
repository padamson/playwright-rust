// DragToOptions and related types
//
// Provides configuration for drag_to actions, matching Playwright's API.

use crate::protocol::click::Position;

/// Options for [`Locator::drag_to()`](crate::protocol::Locator::drag_to).
///
/// Configuration for dragging a source element onto a target element.
///
/// Use the builder pattern to construct options:
///
/// # Example
///
/// ```ignore
/// use playwright_rs::{DragToOptions, Position};
///
/// // Drag with custom source and target positions
/// let options = DragToOptions::builder()
///     .source_position(Position { x: 10.0, y: 10.0 })
///     .target_position(Position { x: 60.0, y: 60.0 })
///     .build();
///
/// // Force drag (bypass actionability checks)
/// let options = DragToOptions::builder()
///     .force(true)
///     .build();
///
/// // Trial run (actionability checks only, don't actually drag)
/// let options = DragToOptions::builder()
///     .trial(true)
///     .build();
/// ```
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-drag-to>
#[derive(Debug, Clone, Default)]
pub struct DragToOptions {
    /// Whether to bypass actionability checks
    pub force: Option<bool>,
    /// Don't wait for navigation after the action
    pub no_wait_after: Option<bool>,
    /// Maximum time in milliseconds
    pub timeout: Option<f64>,
    /// Perform actionability checks without dragging
    pub trial: Option<bool>,
    /// Where to click on the source element (relative to top-left corner)
    pub source_position: Option<Position>,
    /// Where to drop on the target element (relative to top-left corner)
    pub target_position: Option<Position>,
}

impl DragToOptions {
    /// Create a new builder for DragToOptions
    pub fn builder() -> DragToOptionsBuilder {
        DragToOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(force) = self.force {
            json["force"] = serde_json::json!(force);
        }

        if let Some(no_wait_after) = self.no_wait_after {
            json["noWaitAfter"] = serde_json::json!(no_wait_after);
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

        if let Some(source_position) = &self.source_position {
            json["sourcePosition"] = serde_json::to_value(source_position).unwrap();
        }

        if let Some(target_position) = &self.target_position {
            json["targetPosition"] = serde_json::to_value(target_position).unwrap();
        }

        json
    }
}

/// Builder for DragToOptions
///
/// Provides a fluent API for constructing drag_to options.
#[derive(Debug, Clone, Default)]
pub struct DragToOptionsBuilder {
    force: Option<bool>,
    no_wait_after: Option<bool>,
    timeout: Option<f64>,
    trial: Option<bool>,
    source_position: Option<Position>,
    target_position: Option<Position>,
}

impl DragToOptionsBuilder {
    /// Bypass actionability checks
    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    /// Don't wait for navigation after the action
    pub fn no_wait_after(mut self, no_wait_after: bool) -> Self {
        self.no_wait_after = Some(no_wait_after);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Perform actionability checks without dragging
    pub fn trial(mut self, trial: bool) -> Self {
        self.trial = Some(trial);
        self
    }

    /// Set where to click on the source element (relative to top-left corner)
    pub fn source_position(mut self, source_position: Position) -> Self {
        self.source_position = Some(source_position);
        self
    }

    /// Set where to drop on the target element (relative to top-left corner)
    pub fn target_position(mut self, target_position: Position) -> Self {
        self.target_position = Some(target_position);
        self
    }

    /// Build the DragToOptions
    pub fn build(self) -> DragToOptions {
        DragToOptions {
            force: self.force,
            no_wait_after: self.no_wait_after,
            timeout: self.timeout,
            trial: self.trial,
            source_position: self.source_position,
            target_position: self.target_position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drag_to_options_default() {
        let options = DragToOptions::builder().build();
        let json = options.to_json();
        // timeout has a default value
        assert!(json["timeout"].is_number());
        // other optional fields are absent
        assert!(json.get("force").is_none());
        assert!(json.get("trial").is_none());
        assert!(json.get("sourcePosition").is_none());
        assert!(json.get("targetPosition").is_none());
    }

    #[test]
    fn test_drag_to_options_force() {
        let options = DragToOptions::builder().force(true).build();
        let json = options.to_json();
        assert_eq!(json["force"], true);
    }

    #[test]
    fn test_drag_to_options_timeout() {
        let options = DragToOptions::builder().timeout(5000.0).build();
        let json = options.to_json();
        assert_eq!(json["timeout"], 5000.0);
    }

    #[test]
    fn test_drag_to_options_trial() {
        let options = DragToOptions::builder().trial(true).build();
        let json = options.to_json();
        assert_eq!(json["trial"], true);
    }

    #[test]
    fn test_drag_to_options_positions() {
        let options = DragToOptions::builder()
            .source_position(Position { x: 5.0, y: 10.0 })
            .target_position(Position { x: 50.0, y: 60.0 })
            .build();
        let json = options.to_json();
        assert_eq!(json["sourcePosition"]["x"], 5.0);
        assert_eq!(json["sourcePosition"]["y"], 10.0);
        assert_eq!(json["targetPosition"]["x"], 50.0);
        assert_eq!(json["targetPosition"]["y"], 60.0);
    }

    #[test]
    fn test_drag_to_options_no_wait_after() {
        let options = DragToOptions::builder().no_wait_after(true).build();
        let json = options.to_json();
        assert_eq!(json["noWaitAfter"], true);
    }
}
