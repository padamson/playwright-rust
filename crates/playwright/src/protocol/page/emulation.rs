use super::Page;
use crate::error::{Error, Result};
use crate::protocol::browser_context::Viewport;
use crate::server::channel_owner::ChannelOwner;
use serde::Serialize;
use std::sync::Arc;

/// Emulation: media, viewport, and injected style and script tags.
impl Page {
    /// Adds a `<style>` tag into the page with the desired content.
    ///
    /// # Arguments
    ///
    /// * `options` - Style tag options (content, url, or path)
    ///
    /// # Returns
    ///
    /// Returns an ElementHandle pointing to the injected `<style>` tag
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let context = browser.new_context().await?;
    /// # let page = context.new_page().await?;
    /// use playwright_rs::protocol::AddStyleTagOptions;
    ///
    /// // With inline CSS
    /// page.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .content("body { background-color: red; }")
    ///         .build()
    /// ).await?;
    ///
    /// // With external URL
    /// page.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .url("https://example.com/style.css")
    ///         .build()
    /// ).await?;
    ///
    /// // From file
    /// page.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .path("./styles/custom.css")
    ///         .build()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-add-style-tag>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn add_style_tag(
        &self,
        options: AddStyleTagOptions,
    ) -> Result<Arc<crate::protocol::ElementHandle>> {
        let frame = self.main_frame().await?;
        frame.add_style_tag(options).await
    }

    /// Sets the viewport size for the page.
    ///
    /// This method allows dynamic resizing of the viewport after page creation,
    /// useful for testing responsive layouts at different screen sizes.
    ///
    /// # Arguments
    ///
    /// * `viewport` - The viewport dimensions (width and height in pixels)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, Viewport};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// // Set viewport to mobile size
    /// let mobile = Viewport {
    ///     width: 375,
    ///     height: 667,
    /// };
    /// page.set_viewport_size(mobile).await?;
    ///
    /// // Later, test desktop layout
    /// let desktop = Viewport {
    ///     width: 1920,
    ///     height: 1080,
    /// };
    /// page.set_viewport_size(desktop).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-viewport-size>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn set_viewport_size(&self, viewport: crate::protocol::Viewport) -> Result<()> {
        // Store the new viewport locally so viewport_size() can reflect the change
        if let Ok(mut guard) = self.viewport.write() {
            *guard = Some(viewport.clone());
        }
        self.channel()
            .send_no_result(
                "setViewportSize",
                serde_json::json!({ "viewportSize": viewport }),
            )
            .await
    }

    /// Emulates media features for the page.
    ///
    /// This method allows emulating CSS media features such as `media`, `color-scheme`,
    /// `reduced-motion`, and `forced-colors`. Pass `None` to call with no changes.
    ///
    /// To reset a specific feature to the browser default, use the `NoOverride` variant.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional emulation options. If `None`, this is a no-op.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, EmulateMediaOptions, Media, ColorScheme};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// // Emulate print media
    /// page.emulate_media(Some(
    ///     EmulateMediaOptions::builder()
    ///         .media(Media::Print)
    ///         .build()
    /// )).await?;
    ///
    /// // Emulate dark color scheme
    /// page.emulate_media(Some(
    ///     EmulateMediaOptions::builder()
    ///         .color_scheme(ColorScheme::Dark)
    ///         .build()
    /// )).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn emulate_media(
        &self,
        options: impl Into<Option<EmulateMediaOptions>>,
    ) -> Result<()> {
        let options = options.into();
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            if let Some(media) = opts.media {
                params["media"] = serde_json::to_value(media).map_err(|e| {
                    crate::error::Error::ProtocolError(format!("Failed to serialize media: {}", e))
                })?;
            }
            if let Some(color_scheme) = opts.color_scheme {
                params["colorScheme"] = serde_json::to_value(color_scheme).map_err(|e| {
                    crate::error::Error::ProtocolError(format!(
                        "Failed to serialize colorScheme: {}",
                        e
                    ))
                })?;
            }
            if let Some(reduced_motion) = opts.reduced_motion {
                params["reducedMotion"] = serde_json::to_value(reduced_motion).map_err(|e| {
                    crate::error::Error::ProtocolError(format!(
                        "Failed to serialize reducedMotion: {}",
                        e
                    ))
                })?;
            }
            if let Some(forced_colors) = opts.forced_colors {
                params["forcedColors"] = serde_json::to_value(forced_colors).map_err(|e| {
                    crate::error::Error::ProtocolError(format!(
                        "Failed to serialize forcedColors: {}",
                        e
                    ))
                })?;
            }
        }

        self.channel().send_no_result("emulateMedia", params).await
    }

    /// Adds a `<script>` tag into the page with the desired URL or content.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional script tag options (content, url, or path).
    ///   If `None`, returns an error because no source is specified.
    ///
    /// At least one of `content`, `url`, or `path` must be provided.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, AddScriptTagOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let context = browser.new_context().await?;
    /// # let page = context.new_page().await?;
    /// // With inline JavaScript
    /// page.add_script_tag(Some(
    ///     AddScriptTagOptions::builder()
    ///         .content("window.myVar = 42;")
    ///         .build()
    /// )).await?;
    ///
    /// // With external URL
    /// page.add_script_tag(Some(
    ///     AddScriptTagOptions::builder()
    ///         .url("https://example.com/script.js")
    ///         .build()
    /// )).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `options` is `None` or no content/url/path is specified
    /// - Page has been closed
    /// - Script loading fails (e.g., invalid URL)
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-add-script-tag>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn add_script_tag(
        &self,
        options: impl Into<Option<AddScriptTagOptions>>,
    ) -> Result<Arc<crate::protocol::ElementHandle>> {
        let options = options.into();
        let opts = options.ok_or_else(|| {
            Error::InvalidArgument(
                "At least one of content, url, or path must be specified".to_string(),
            )
        })?;
        let frame = self.main_frame().await?;
        frame.add_script_tag(opts).await
    }

    /// Returns the current viewport size of the page, or `None` if no viewport is set.
    ///
    /// Returns `None` when the context was created with `no_viewport: true`. Otherwise
    /// returns the dimensions configured at context creation time or updated via
    /// `set_viewport_size()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, BrowserContextOptions, Viewport};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// let context = browser.new_context_with_options(
    ///     BrowserContextOptions::builder().viewport(Viewport { width: 1280, height: 720 }).build()
    /// ).await?;
    /// let page = context.new_page().await?;
    /// let size = page.viewport_size().expect("Viewport should be set");
    /// assert_eq!(size.width, 1280);
    /// assert_eq!(size.height, 720);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-viewport-size>
    pub fn viewport_size(&self) -> Option<Viewport> {
        self.viewport.read().ok()?.clone()
    }
}

/// Options for adding a style tag to the page
///
/// See: <https://playwright.dev/docs/api/class-page#page-add-style-tag>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct AddStyleTagOptions {
    /// Raw CSS content to inject
    pub content: Option<String>,
    /// URL of the `<link>` tag to add
    pub url: Option<String>,
    /// Path to a CSS file to inject
    pub path: Option<String>,
}

impl AddStyleTagOptions {
    /// Creates a new builder for AddStyleTagOptions
    pub fn builder() -> AddStyleTagOptionsBuilder {
        AddStyleTagOptionsBuilder::default()
    }

    /// Validates that at least one option is specified
    pub(crate) fn validate(&self) -> Result<()> {
        if self.content.is_none() && self.url.is_none() && self.path.is_none() {
            return Err(Error::InvalidArgument(
                "At least one of content, url, or path must be specified".to_string(),
            ));
        }
        Ok(())
    }
}

/// Builder for AddStyleTagOptions
#[derive(Debug, Clone, Default)]
pub struct AddStyleTagOptionsBuilder {
    content: Option<String>,
    url: Option<String>,
    path: Option<String>,
}

impl AddStyleTagOptionsBuilder {
    /// Sets the CSS content to inject
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Sets the URL of the stylesheet
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the path to a CSS file
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Builds the AddStyleTagOptions
    pub fn build(self) -> AddStyleTagOptions {
        AddStyleTagOptions {
            content: self.content,
            url: self.url,
            path: self.path,
        }
    }
}

// ============================================================================
// AddScriptTagOptions
// ============================================================================

/// Options for adding a `<script>` tag to the page.
///
/// At least one of `content`, `url`, or `path` must be specified.
///
/// See: <https://playwright.dev/docs/api/class-page#page-add-script-tag>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct AddScriptTagOptions {
    /// Raw JavaScript content to inject
    pub content: Option<String>,
    /// URL of the `<script>` tag to add
    pub url: Option<String>,
    /// Path to a JavaScript file to inject (file contents will be read and sent as content)
    pub path: Option<String>,
    /// Script type attribute (e.g., `"module"`)
    pub type_: Option<String>,
}

impl AddScriptTagOptions {
    /// Creates a new builder for AddScriptTagOptions
    pub fn builder() -> AddScriptTagOptionsBuilder {
        AddScriptTagOptionsBuilder::default()
    }

    /// Validates that at least one option is specified
    pub(crate) fn validate(&self) -> Result<()> {
        if self.content.is_none() && self.url.is_none() && self.path.is_none() {
            return Err(Error::InvalidArgument(
                "At least one of content, url, or path must be specified".to_string(),
            ));
        }
        Ok(())
    }
}

/// Builder for AddScriptTagOptions
#[derive(Debug, Clone, Default)]
pub struct AddScriptTagOptionsBuilder {
    content: Option<String>,
    url: Option<String>,
    path: Option<String>,
    type_: Option<String>,
}

impl AddScriptTagOptionsBuilder {
    /// Sets the JavaScript content to inject
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Sets the URL of the script to load
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the path to a JavaScript file to inject
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Sets the script type attribute (e.g., `"module"`)
    pub fn type_(mut self, type_: impl Into<String>) -> Self {
        self.type_ = Some(type_.into());
        self
    }

    /// Builds the AddScriptTagOptions
    pub fn build(self) -> AddScriptTagOptions {
        AddScriptTagOptions {
            content: self.content,
            url: self.url,
            path: self.path,
            type_: self.type_,
        }
    }
}

// ============================================================================
// EmulateMediaOptions and related enums
// ============================================================================

/// Media type for `page.emulate_media()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Media {
    /// Emulate screen media type
    Screen,
    /// Emulate print media type
    Print,
    /// Reset media emulation to browser default (sends `"no-override"` to protocol)
    #[serde(rename = "no-override")]
    NoOverride,
}

/// Preferred color scheme for `page.emulate_media()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum ColorScheme {
    /// Emulate light color scheme
    #[serde(rename = "light")]
    Light,
    /// Emulate dark color scheme
    #[serde(rename = "dark")]
    Dark,
    /// Emulate no preference for color scheme
    #[serde(rename = "no-preference")]
    NoPreference,
    /// Reset color scheme to browser default
    #[serde(rename = "no-override")]
    NoOverride,
}

/// Reduced motion preference for `page.emulate_media()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum ReducedMotion {
    /// Emulate reduced motion preference
    #[serde(rename = "reduce")]
    Reduce,
    /// Emulate no preference for reduced motion
    #[serde(rename = "no-preference")]
    NoPreference,
    /// Reset reduced motion to browser default
    #[serde(rename = "no-override")]
    NoOverride,
}

/// Forced colors preference for `page.emulate_media()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum ForcedColors {
    /// Emulate active forced colors
    #[serde(rename = "active")]
    Active,
    /// Emulate no forced colors
    #[serde(rename = "none")]
    None_,
    /// Reset forced colors to browser default
    #[serde(rename = "no-override")]
    NoOverride,
}

/// Options for `page.emulate_media()`.
///
/// All fields are optional. Fields that are `None` are omitted from the protocol
/// message (meaning they are not changed). To reset a field to browser default,
/// use the `NoOverride` variant.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct EmulateMediaOptions {
    /// Media type to emulate (screen, print, or no-override)
    pub media: Option<Media>,
    /// Color scheme preference to emulate
    pub color_scheme: Option<ColorScheme>,
    /// Reduced motion preference to emulate
    pub reduced_motion: Option<ReducedMotion>,
    /// Forced colors preference to emulate
    pub forced_colors: Option<ForcedColors>,
}

impl EmulateMediaOptions {
    /// Creates a new builder for EmulateMediaOptions
    pub fn builder() -> EmulateMediaOptionsBuilder {
        EmulateMediaOptionsBuilder::default()
    }
}

/// Builder for EmulateMediaOptions
#[derive(Debug, Clone, Default)]
pub struct EmulateMediaOptionsBuilder {
    media: Option<Media>,
    color_scheme: Option<ColorScheme>,
    reduced_motion: Option<ReducedMotion>,
    forced_colors: Option<ForcedColors>,
}

impl EmulateMediaOptionsBuilder {
    /// Sets the media type to emulate
    pub fn media(mut self, media: Media) -> Self {
        self.media = Some(media);
        self
    }

    /// Sets the color scheme preference
    pub fn color_scheme(mut self, color_scheme: ColorScheme) -> Self {
        self.color_scheme = Some(color_scheme);
        self
    }

    /// Sets the reduced motion preference
    pub fn reduced_motion(mut self, reduced_motion: ReducedMotion) -> Self {
        self.reduced_motion = Some(reduced_motion);
        self
    }

    /// Sets the forced colors preference
    pub fn forced_colors(mut self, forced_colors: ForcedColors) -> Self {
        self.forced_colors = Some(forced_colors);
        self
    }

    /// Builds the EmulateMediaOptions
    pub fn build(self) -> EmulateMediaOptions {
        EmulateMediaOptions {
            media: self.media,
            color_scheme: self.color_scheme,
            reduced_motion: self.reduced_motion,
            forced_colors: self.forced_colors,
        }
    }
}
