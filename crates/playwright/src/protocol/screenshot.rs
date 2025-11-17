// Screenshot types and options
//
// Provides configuration for page and element screenshots, matching Playwright's API.

use serde::Serialize;

/// Screenshot image format
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::ScreenshotType;
///
/// let screenshot_type = ScreenshotType::Jpeg;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenshotType {
    /// PNG format (lossless, supports transparency)
    Png,
    /// JPEG format (lossy compression, smaller file size)
    Jpeg,
}

/// Clip region for screenshot
///
/// Specifies a rectangular region to capture.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::ScreenshotClip;
///
/// let clip = ScreenshotClip {
///     x: 10.0,
///     y: 20.0,
///     width: 300.0,
///     height: 200.0,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ScreenshotClip {
    /// X coordinate of clip region origin
    pub x: f64,
    /// Y coordinate of clip region origin
    pub y: f64,
    /// Width of clip region
    pub width: f64,
    /// Height of clip region
    pub height: f64,
}

/// Screenshot options
///
/// Configuration options for page and element screenshots.
///
/// Use the builder pattern to construct options:
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::{ScreenshotOptions, ScreenshotType, ScreenshotClip};
///
/// // JPEG with quality
/// let options = ScreenshotOptions::builder()
///     .screenshot_type(ScreenshotType::Jpeg)
///     .quality(80)
///     .build();
///
/// // Full page screenshot
/// let options = ScreenshotOptions::builder()
///     .full_page(true)
///     .build();
///
/// // Clip region
/// let clip = ScreenshotClip {
///     x: 10.0,
///     y: 10.0,
///     width: 200.0,
///     height: 100.0,
/// };
/// let options = ScreenshotOptions::builder()
///     .clip(clip)
///     .build();
/// ```
///
/// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
#[derive(Debug, Clone, Default)]
pub struct ScreenshotOptions {
    /// Image format (png or jpeg)
    pub screenshot_type: Option<ScreenshotType>,
    /// JPEG quality (0-100), only applies to jpeg format
    pub quality: Option<u8>,
    /// Capture full scrollable page
    pub full_page: Option<bool>,
    /// Clip region to capture
    pub clip: Option<ScreenshotClip>,
    /// Hide default white background (PNG only)
    pub omit_background: Option<bool>,
    /// Screenshot timeout in milliseconds
    pub timeout: Option<f64>,
}

impl ScreenshotOptions {
    /// Create a new builder for ScreenshotOptions
    pub fn builder() -> ScreenshotOptionsBuilder {
        ScreenshotOptionsBuilder::default()
    }

    /// Convert options to JSON value for protocol
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(screenshot_type) = &self.screenshot_type {
            json["type"] = serde_json::to_value(screenshot_type).unwrap();
        }

        if let Some(quality) = self.quality {
            json["quality"] = serde_json::json!(quality);
        }

        if let Some(full_page) = self.full_page {
            json["fullPage"] = serde_json::json!(full_page);
        }

        if let Some(clip) = &self.clip {
            json["clip"] = serde_json::to_value(clip).unwrap();
        }

        if let Some(omit_background) = self.omit_background {
            json["omitBackground"] = serde_json::json!(omit_background);
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

/// Builder for ScreenshotOptions
///
/// Provides a fluent API for constructing screenshot options.
#[derive(Debug, Clone, Default)]
pub struct ScreenshotOptionsBuilder {
    screenshot_type: Option<ScreenshotType>,
    quality: Option<u8>,
    full_page: Option<bool>,
    clip: Option<ScreenshotClip>,
    omit_background: Option<bool>,
    timeout: Option<f64>,
}

impl ScreenshotOptionsBuilder {
    /// Set the screenshot format (png or jpeg)
    pub fn screenshot_type(mut self, screenshot_type: ScreenshotType) -> Self {
        self.screenshot_type = Some(screenshot_type);
        self
    }

    /// Set JPEG quality (0-100)
    ///
    /// Only applies when screenshot_type is Jpeg.
    pub fn quality(mut self, quality: u8) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Capture full scrollable page beyond viewport
    pub fn full_page(mut self, full_page: bool) -> Self {
        self.full_page = Some(full_page);
        self
    }

    /// Set clip region to capture
    pub fn clip(mut self, clip: ScreenshotClip) -> Self {
        self.clip = Some(clip);
        self
    }

    /// Hide default white background (creates transparent PNG)
    pub fn omit_background(mut self, omit_background: bool) -> Self {
        self.omit_background = Some(omit_background);
        self
    }

    /// Set screenshot timeout in milliseconds
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the ScreenshotOptions
    pub fn build(self) -> ScreenshotOptions {
        ScreenshotOptions {
            screenshot_type: self.screenshot_type,
            quality: self.quality,
            full_page: self.full_page,
            clip: self.clip,
            omit_background: self.omit_background,
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshot_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ScreenshotType::Png).unwrap(),
            "\"png\""
        );
        assert_eq!(
            serde_json::to_string(&ScreenshotType::Jpeg).unwrap(),
            "\"jpeg\""
        );
    }

    #[test]
    fn test_builder_jpeg_with_quality() {
        let options = ScreenshotOptions::builder()
            .screenshot_type(ScreenshotType::Jpeg)
            .quality(80)
            .build();

        let json = options.to_json();
        assert_eq!(json["type"], "jpeg");
        assert_eq!(json["quality"], 80);
    }

    #[test]
    fn test_builder_full_page() {
        let options = ScreenshotOptions::builder().full_page(true).build();

        let json = options.to_json();
        assert_eq!(json["fullPage"], true);
    }

    #[test]
    fn test_builder_clip() {
        let clip = ScreenshotClip {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 200.0,
        };
        let options = ScreenshotOptions::builder().clip(clip).build();

        let json = options.to_json();
        assert_eq!(json["clip"]["x"], 10.0);
        assert_eq!(json["clip"]["y"], 20.0);
        assert_eq!(json["clip"]["width"], 300.0);
        assert_eq!(json["clip"]["height"], 200.0);
    }

    #[test]
    fn test_builder_omit_background() {
        let options = ScreenshotOptions::builder().omit_background(true).build();

        let json = options.to_json();
        assert_eq!(json["omitBackground"], true);
    }

    #[test]
    fn test_builder_multiple_options() {
        let options = ScreenshotOptions::builder()
            .screenshot_type(ScreenshotType::Jpeg)
            .quality(90)
            .full_page(true)
            .timeout(5000.0)
            .build();

        let json = options.to_json();
        assert_eq!(json["type"], "jpeg");
        assert_eq!(json["quality"], 90);
        assert_eq!(json["fullPage"], true);
        assert_eq!(json["timeout"], 5000.0);
    }
}
