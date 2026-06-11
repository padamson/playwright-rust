// Screenshot types and options
//
// Provides configuration for page and element screenshots, matching Playwright's API.

use serde::Serialize;

/// Whether to play or freeze CSS animations and transitions during capture.
///
/// Used by [`ScreenshotOptions`] and by the `to_have_screenshot` visual
/// assertions. `Disabled` is the value to use for stable screenshots.
///
/// See: <https://playwright.dev/docs/api/class-page#page-screenshot-option-animations>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Animations {
    /// Allow animations to run normally.
    Allow,
    /// Disable CSS animations and transitions before capturing.
    Disabled,
}

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
#[non_exhaustive]
pub enum ScreenshotType {
    /// PNG format (lossless, supports transparency)
    Png,
    /// JPEG format (lossy compression, smaller file size)
    Jpeg,
}

/// Text-caret handling during screenshot capture.
///
/// See: <https://playwright.dev/docs/api/class-page#page-screenshot-option-caret>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Caret {
    /// Hide the text caret before capturing (Playwright's default).
    Hide,
    /// Leave the caret untouched.
    Initial,
}

/// Pixel scale for the captured image.
///
/// See: <https://playwright.dev/docs/api/class-page#page-screenshot-option-scale>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Scale {
    /// One pixel per CSS pixel; keeps screenshots small (Playwright's default).
    Css,
    /// One pixel per device pixel; sharper on HiDPI displays.
    Device,
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
/// use playwright_rs::Animations;
///
/// // JPEG with quality
/// let options = ScreenshotOptions::builder()
///     .screenshot_type(ScreenshotType::Jpeg)
///     .quality(80)
///     .build();
///
/// // Stable screenshot: freeze animations and hide the caret
/// let options = ScreenshotOptions::builder()
///     .animations(Animations::Disabled)
///     .build();
/// ```
///
/// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
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
    /// Freeze CSS animations and transitions before capturing (stable shots)
    pub animations: Option<Animations>,
    /// Hide or keep the text caret
    pub caret: Option<Caret>,
    /// CSS-pixel vs device-pixel scale
    pub scale: Option<Scale>,
    /// CSS to inject into the page before capturing (e.g. hide dynamic elements)
    pub style: Option<String>,
    /// Locators to mask out (overpaint) with a solid box, pre-serialized to the
    /// protocol `{ frame, selector }` shape. Build via the builder's `mask`.
    pub mask: Option<Vec<serde_json::Value>>,
    /// CSS color of the mask boxes (e.g. `"#FF00FF"`); Playwright defaults to pink.
    pub mask_color: Option<String>,
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
            json["type"] = serde_json::to_value(screenshot_type)
                .expect("serialization of ScreenshotType cannot fail");
        }

        if let Some(quality) = self.quality {
            json["quality"] = serde_json::json!(quality);
        }

        if let Some(full_page) = self.full_page {
            json["fullPage"] = serde_json::json!(full_page);
        }

        if let Some(clip) = &self.clip {
            json["clip"] = serde_json::to_value(clip).expect("serialization of clip cannot fail");
        }

        if let Some(omit_background) = self.omit_background {
            json["omitBackground"] = serde_json::json!(omit_background);
        }

        if let Some(animations) = &self.animations {
            json["animations"] =
                serde_json::to_value(animations).expect("serialization of Animations cannot fail");
        }

        if let Some(caret) = &self.caret {
            json["caret"] =
                serde_json::to_value(caret).expect("serialization of Caret cannot fail");
        }

        if let Some(scale) = &self.scale {
            json["scale"] =
                serde_json::to_value(scale).expect("serialization of Scale cannot fail");
        }

        if let Some(style) = &self.style {
            json["style"] = serde_json::json!(style);
        }

        if let Some(mask) = &self.mask {
            json["mask"] = serde_json::Value::Array(mask.clone());
        }

        if let Some(mask_color) = &self.mask_color {
            json["maskColor"] = serde_json::json!(mask_color);
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
    animations: Option<Animations>,
    caret: Option<Caret>,
    scale: Option<Scale>,
    style: Option<String>,
    mask: Option<Vec<serde_json::Value>>,
    mask_color: Option<String>,
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

    /// Freeze CSS animations and transitions before capturing.
    ///
    /// Use [`Animations::Disabled`] for stable screenshots (the value
    /// Playwright's own visual assertions use).
    pub fn animations(mut self, animations: Animations) -> Self {
        self.animations = Some(animations);
        self
    }

    /// Hide or keep the text caret during capture.
    pub fn caret(mut self, caret: Caret) -> Self {
        self.caret = Some(caret);
        self
    }

    /// Capture at CSS-pixel or device-pixel scale.
    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = Some(scale);
        self
    }

    /// Inject a CSS stylesheet into the page before capturing.
    pub fn style(mut self, style: impl Into<String>) -> Self {
        self.style = Some(style.into());
        self
    }

    /// Overpaint the given locators with a solid box (mask out dynamic or
    /// sensitive content). Each locator may match multiple elements; all are
    /// masked. Pair with [`mask_color`](Self::mask_color) to set the box color.
    pub fn mask(mut self, mask: Vec<crate::protocol::Locator>) -> Self {
        self.mask = Some(mask.iter().map(|l| l.mask_json()).collect());
        self
    }

    /// CSS color of the [`mask`](Self::mask) boxes (e.g. `"#FF00FF"` or
    /// `"rgba(0,0,0,0.5)"`). Defaults to pink when unset.
    pub fn mask_color(mut self, mask_color: impl Into<String>) -> Self {
        self.mask_color = Some(mask_color.into());
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
            animations: self.animations,
            caret: self.caret,
            scale: self.scale,
            style: self.style,
            mask: self.mask,
            mask_color: self.mask_color,
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
    fn test_builder_animations() {
        let json = ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .build()
            .to_json();
        assert_eq!(json["animations"], "disabled");

        let json = ScreenshotOptions::builder()
            .animations(Animations::Allow)
            .build()
            .to_json();
        assert_eq!(json["animations"], "allow");
    }

    #[test]
    fn test_builder_caret() {
        let json = ScreenshotOptions::builder()
            .caret(Caret::Hide)
            .build()
            .to_json();
        assert_eq!(json["caret"], "hide");
    }

    #[test]
    fn test_builder_scale() {
        let json = ScreenshotOptions::builder()
            .scale(Scale::Device)
            .build()
            .to_json();
        assert_eq!(json["scale"], "device");
    }

    #[test]
    fn test_builder_style() {
        let json = ScreenshotOptions::builder()
            .style(".flaky { visibility: hidden; }")
            .build()
            .to_json();
        assert_eq!(json["style"], ".flaky { visibility: hidden; }");
    }

    #[test]
    fn test_builder_mask_color() {
        let json = ScreenshotOptions::builder()
            .mask_color("#FF00FF")
            .build()
            .to_json();
        assert_eq!(json["maskColor"], "#FF00FF");
    }

    #[test]
    fn test_mask_serializes_to_array() {
        // The builder's `mask` needs a real Locator (browser-only), so construct
        // the pre-serialized form directly to lock the `to_json` mask branch.
        let options = ScreenshotOptions {
            mask: Some(vec![serde_json::json!({
                "frame": { "guid": "frame@1" },
                "selector": "h1",
            })]),
            ..Default::default()
        };
        let json = options.to_json();
        assert_eq!(json["mask"][0]["frame"]["guid"], "frame@1");
        assert_eq!(json["mask"][0]["selector"], "h1");
    }

    #[test]
    fn test_unset_options_absent() {
        let json = ScreenshotOptions::builder().build().to_json();
        assert!(json.get("animations").is_none());
        assert!(json.get("caret").is_none());
        assert!(json.get("scale").is_none());
        assert!(json.get("style").is_none());
        assert!(json.get("mask").is_none());
        assert!(json.get("maskColor").is_none());
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
