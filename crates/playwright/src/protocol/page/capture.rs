use super::{Page, ScreencastFrameHandler, ScreencastFrameHandlerFuture};
use crate::error::Result;
use crate::server::channel_owner::ChannelOwner;
use crate::server::connection::ConnectionExt;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::Arc;

/// Capture: screenshots, PDF, screencast, coverage and accessibility snapshots.
impl Page {
    /// Takes a screenshot of the page and returns the image bytes.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid(), bytes_len = tracing::field::Empty))]
    pub async fn screenshot(
        &self,
        options: impl Into<Option<crate::protocol::ScreenshotOptions>>,
    ) -> Result<Vec<u8>> {
        let options = options.into();
        let params = if let Some(opts) = options {
            opts.to_json()
        } else {
            // Default to PNG with required timeout
            serde_json::json!({
                "type": "png",
                "timeout": crate::DEFAULT_TIMEOUT_MS
            })
        };

        #[derive(Deserialize)]
        struct ScreenshotResponse {
            binary: String,
        }

        let response: ScreenshotResponse = self.channel().send("screenshot", params).await?;

        // Decode base64 to bytes
        let bytes = base64::prelude::BASE64_STANDARD
            .decode(&response.binary)
            .map_err(|e| {
                crate::error::Error::ProtocolError(format!("Failed to decode screenshot: {}", e))
            })?;

        tracing::Span::current().record("bytes_len", bytes.len());
        Ok(bytes)
    }

    /// Takes a screenshot and saves it to a file, also returning the bytes.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn screenshot_to_file(
        &self,
        path: &std::path::Path,
        options: impl Into<Option<crate::protocol::ScreenshotOptions>>,
    ) -> Result<Vec<u8>> {
        let options = options.into();
        // Get the screenshot bytes
        let bytes = self.screenshot(options).await?;

        // Write to file
        tokio::fs::write(path, &bytes).await.map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to write screenshot file: {}", e))
        })?;

        Ok(bytes)
    }

    /// Generates a PDF of the page and returns it as bytes.
    ///
    /// Note: Generating a PDF is only supported in Chromium headless. PDF generation is
    /// not supported in Firefox or WebKit.
    ///
    /// The PDF bytes are returned. If `options.path` is set, the PDF will also be
    /// saved to that file.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional PDF generation options
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// let pdf_bytes = page.pdf(None).await?;
    /// assert!(!pdf_bytes.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The browser is not Chromium (PDF only supported in Chromium)
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-pdf>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid(), bytes_len = tracing::field::Empty))]
    pub async fn pdf(&self, options: impl Into<Option<PdfOptions>>) -> Result<Vec<u8>> {
        let options = options.into();
        let mut params = serde_json::json!({});
        let mut save_path: Option<std::path::PathBuf> = None;

        if let Some(opts) = options {
            // Capture the file path before consuming opts
            save_path = opts.path;

            if let Some(scale) = opts.scale {
                params["scale"] = serde_json::json!(scale);
            }
            if let Some(v) = opts.display_header_footer {
                params["displayHeaderFooter"] = serde_json::json!(v);
            }
            if let Some(v) = opts.header_template {
                params["headerTemplate"] = serde_json::json!(v);
            }
            if let Some(v) = opts.footer_template {
                params["footerTemplate"] = serde_json::json!(v);
            }
            if let Some(v) = opts.print_background {
                params["printBackground"] = serde_json::json!(v);
            }
            if let Some(v) = opts.landscape {
                params["landscape"] = serde_json::json!(v);
            }
            if let Some(v) = opts.page_ranges {
                params["pageRanges"] = serde_json::json!(v);
            }
            if let Some(v) = opts.format {
                params["format"] = serde_json::json!(v);
            }
            if let Some(v) = opts.width {
                params["width"] = serde_json::json!(v);
            }
            if let Some(v) = opts.height {
                params["height"] = serde_json::json!(v);
            }
            if let Some(v) = opts.prefer_css_page_size {
                params["preferCSSPageSize"] = serde_json::json!(v);
            }
            if let Some(margin) = opts.margin {
                params["margin"] = serde_json::to_value(margin).map_err(|e| {
                    crate::error::Error::ProtocolError(format!("Failed to serialize margin: {}", e))
                })?;
            }
        }

        #[derive(Deserialize)]
        struct PdfResponse {
            pdf: String,
        }

        let response: PdfResponse = self.channel().send("pdf", params).await?;

        // Decode base64 to bytes
        let pdf_bytes = base64::engine::general_purpose::STANDARD
            .decode(&response.pdf)
            .map_err(|e| {
                crate::error::Error::ProtocolError(format!("Failed to decode PDF base64: {}", e))
            })?;

        // If a path was specified, save the PDF to disk as well
        if let Some(path) = save_path {
            tokio::fs::write(&path, &pdf_bytes).await.map_err(|e| {
                crate::error::Error::InvalidArgument(format!(
                    "Failed to write PDF to '{}': {}",
                    path.display(),
                    e
                ))
            })?;
        }

        tracing::Span::current().record("bytes_len", pdf_bytes.len());
        Ok(pdf_bytes)
    }

    /// Returns the `Accessibility` object for this page.
    ///
    /// Use `accessibility().snapshot()` to capture the current state of the
    /// page's accessibility tree.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-accessibility>
    pub fn accessibility(&self) -> crate::protocol::Accessibility {
        crate::protocol::Accessibility::new(self.clone())
    }

    /// Returns the ARIA accessibility tree for the page as a YAML string.
    ///
    /// Page-level shorthand for `page.locator("body").aria_snapshot(...)`. Useful
    /// for asserting page-wide accessibility structure without first selecting
    /// `body` explicitly.
    ///
    /// Pass `Some(AriaSnapshotOptions::default().mode(AriaSnapshotMode::Ai))`
    /// to get the AI-friendly form intended for LLM/codegen consumption.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-aria-snapshot>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn aria_snapshot(
        &self,
        options: impl Into<Option<crate::protocol::AriaSnapshotOptions>>,
    ) -> Result<String> {
        let options = options.into();
        let frame = self.main_frame().await?;
        let timeout = options
            .as_ref()
            .and_then(|o| o.timeout)
            .unwrap_or_else(|| self.default_timeout_ms());
        frame
            .aria_snapshot_raw("body", timeout, options.as_ref())
            .await
    }

    /// The whole document's accessibility tree, as JSON rather than the YAML
    /// markup [`aria_snapshot`](Self::aria_snapshot) returns, so a caller can
    /// walk it instead of parsing text.
    ///
    /// # Errors
    ///
    /// Returns an error if the driver rejects the request or the page closes
    /// first.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-aria-snapshot-json>
    pub async fn aria_snapshot_json(
        &self,
        options: impl Into<Option<crate::protocol::AriaSnapshotOptions>>,
    ) -> Result<serde_json::Value> {
        let options = options.into();
        let frame = self.main_frame().await?;
        let timeout = options
            .as_ref()
            .and_then(|o| o.timeout)
            .unwrap_or_else(|| self.default_timeout_ms());
        frame
            .aria_snapshot_json_raw("body", timeout, options.as_ref())
            .await
    }

    /// Returns the `Coverage` object for this page (Chromium only).
    ///
    /// Use `coverage().start_js_coverage()` / `stop_js_coverage()` and
    /// `start_css_coverage()` / `stop_css_coverage()` to collect code coverage data.
    ///
    /// Coverage is only available in Chromium. Calling coverage methods on
    /// Firefox or WebKit will return an error from the Playwright server.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-coverage>
    pub fn coverage(&self) -> crate::protocol::Coverage {
        crate::protocol::Coverage::new(self.clone())
    }

    /// Returns the live-screencast handle for this page.
    ///
    /// Register frame handlers via [`Screencast::on_frame`](crate::Screencast::on_frame), then call
    /// [`Screencast::start`](crate::Screencast::start) to begin streaming. JPEG frames arrive on
    /// the registered handlers as the browser renders.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-screencast>
    pub fn screencast(&self) -> crate::protocol::Screencast {
        crate::protocol::Screencast::new(self.clone())
    }

    pub(crate) async fn screencast_start(
        &self,
        options: crate::protocol::ScreencastStartOptions,
    ) -> Result<()> {
        let mut params = serde_json::json!({});
        if let Some(size) = options.size {
            params["size"] = serde_json::json!({
                "width": size.width,
                "height": size.height,
            });
        }
        if let Some(quality) = options.quality {
            params["quality"] = serde_json::json!(quality);
        }
        let has_handlers = !self.screencast_frame_handlers.lock().unwrap().is_empty();
        params["sendFrames"] = serde_json::json!(has_handlers);
        let recording = options.path.is_some();
        params["record"] = serde_json::json!(recording);

        #[derive(serde::Deserialize)]
        struct StartResponse {
            artifact: Option<serde_json::Value>,
        }
        let response: StartResponse = self.channel().send("screencastStart", params).await?;

        if recording {
            *self.screencast_save_path.lock().unwrap() = options.path;
            if let Some(artifact_value) = response.artifact
                && let Some(guid) = artifact_value.get("guid").and_then(|v| v.as_str())
            {
                *self.screencast_artifact_guid.lock().unwrap() = Some(guid.to_string());
            }
        }
        Ok(())
    }

    pub(crate) async fn screencast_stop(&self) -> Result<()> {
        self.channel()
            .send_no_result("screencastStop", serde_json::json!({}))
            .await?;

        let path = self.screencast_save_path.lock().unwrap().take();
        let artifact_guid = self.screencast_artifact_guid.lock().unwrap().take();
        if let (Some(path), Some(guid)) = (path, artifact_guid) {
            let artifact = self
                .connection()
                .get_typed::<crate::protocol::artifact::Artifact>(&guid)
                .await?;
            artifact.save_as(path.to_string_lossy().as_ref()).await?;
        }
        Ok(())
    }

    pub(crate) fn screencast_on_frame<F, Fut>(&self, handler: F)
    where
        F: Fn(crate::protocol::ScreencastFrame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let h: ScreencastFrameHandler = Arc::new(
            move |f: crate::protocol::ScreencastFrame| -> ScreencastFrameHandlerFuture {
                Box::pin(handler(f))
            },
        );
        self.screencast_frame_handlers.lock().unwrap().push(h);
    }

    pub(crate) async fn screencast_show_actions(
        &self,
        options: crate::protocol::ShowActionsOptions,
    ) -> Result<()> {
        let mut params = serde_json::json!({});
        if let Some(d) = options.duration {
            params["duration"] = serde_json::json!(d);
        }
        if let Some(p) = options.position {
            params["position"] = serde_json::json!(p.as_str());
        }
        if let Some(f) = options.font_size {
            params["fontSize"] = serde_json::json!(f);
        }
        if let Some(c) = options.cursor {
            params["cursor"] = serde_json::json!(c.as_str());
        }
        self.channel()
            .send_no_result("screencastShowActions", params)
            .await
    }

    pub(crate) async fn screencast_hide_actions(&self) -> Result<()> {
        self.channel()
            .send_no_result("screencastHideActions", serde_json::json!({}))
            .await
    }

    pub(crate) async fn screencast_chapter(
        &self,
        title: &str,
        options: crate::protocol::ChapterOptions,
    ) -> Result<()> {
        let mut params = serde_json::json!({ "title": title });
        if let Some(desc) = options.description {
            params["description"] = serde_json::json!(desc);
        }
        if let Some(d) = options.duration {
            params["duration"] = serde_json::json!(d);
        }
        self.channel()
            .send_no_result("screencastChapter", params)
            .await
    }

    pub(crate) async fn screencast_show_overlay(
        &self,
        html: &str,
        options: crate::protocol::ShowOverlayOptions,
    ) -> Result<crate::protocol::OverlayId> {
        let mut params = serde_json::json!({ "html": html });
        if let Some(d) = options.duration {
            params["duration"] = serde_json::json!(d);
        }
        #[derive(serde::Deserialize)]
        struct OverlayResponse {
            id: String,
        }
        let response: OverlayResponse =
            self.channel().send("screencastShowOverlay", params).await?;
        Ok(crate::protocol::OverlayId(response.id))
    }

    pub(crate) async fn screencast_remove_overlay(
        &self,
        id: crate::protocol::OverlayId,
    ) -> Result<()> {
        self.channel()
            .send_no_result("screencastRemoveOverlay", serde_json::json!({ "id": id.0 }))
            .await
    }

    pub(crate) async fn screencast_set_overlay_visible(&self, visible: bool) -> Result<()> {
        self.channel()
            .send_no_result(
                "screencastSetOverlayVisible",
                serde_json::json!({ "visible": visible }),
            )
            .await
    }

    // Internal accessibility method (called by Accessibility struct)
    //
    // The legacy `accessibilitySnapshot` RPC was removed in modern Playwright.
    // We implement snapshot() using `FrameAriaSnapshot` on the main frame, which
    // returns the ARIA accessibility tree as a YAML string (the current equivalent).
    // The YAML string is returned as a JSON string Value for API compatibility.

    pub(crate) async fn accessibility_snapshot(
        &self,
        _options: Option<crate::protocol::accessibility::AccessibilitySnapshotOptions>,
    ) -> Result<serde_json::Value> {
        let frame = self.main_frame().await?;
        let timeout = self.default_timeout_ms();
        let snapshot = frame.aria_snapshot_raw("body", timeout, None).await?;
        Ok(serde_json::Value::String(snapshot))
    }

    // Internal coverage methods (called by Coverage struct)

    pub(crate) async fn coverage_start_js(
        &self,
        options: Option<crate::protocol::coverage::StartJSCoverageOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            if let Some(v) = opts.reset_on_navigation {
                params["resetOnNavigation"] = serde_json::json!(v);
            }
            if let Some(v) = opts.report_anonymous_scripts {
                params["reportAnonymousScripts"] = serde_json::json!(v);
            }
        }

        self.channel()
            .send_no_result("startJSCoverage", params)
            .await
    }

    pub(crate) async fn coverage_stop_js(
        &self,
    ) -> Result<Vec<crate::protocol::coverage::JSCoverageEntry>> {
        #[derive(serde::Deserialize)]
        struct StopJSCoverageResponse {
            entries: Vec<crate::protocol::coverage::JSCoverageEntry>,
        }

        let response: StopJSCoverageResponse = self
            .channel()
            .send("stopJSCoverage", serde_json::json!({}))
            .await?;

        Ok(response.entries)
    }

    pub(crate) async fn coverage_start_css(
        &self,
        options: Option<crate::protocol::coverage::StartCSSCoverageOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options
            && let Some(v) = opts.reset_on_navigation
        {
            params["resetOnNavigation"] = serde_json::json!(v);
        }

        self.channel()
            .send_no_result("startCSSCoverage", params)
            .await
    }

    pub(crate) async fn coverage_stop_css(
        &self,
    ) -> Result<Vec<crate::protocol::coverage::CSSCoverageEntry>> {
        #[derive(serde::Deserialize)]
        struct StopCSSCoverageResponse {
            entries: Vec<crate::protocol::coverage::CSSCoverageEntry>,
        }

        let response: StopCSSCoverageResponse = self
            .channel()
            .send("stopCSSCoverage", serde_json::json!({}))
            .await?;

        Ok(response.entries)
    }
}

/// Margin options for PDF generation.
///
/// See: <https://playwright.dev/docs/api/class-page#page-pdf>
#[derive(Debug, Clone, Default, Serialize)]
pub struct PdfMargin {
    /// Top margin (e.g. `"1in"`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<String>,
    /// Right margin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<String>,
    /// Bottom margin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<String>,
    /// Left margin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<String>,
}

/// Options for generating a PDF from a page.
///
/// Note: PDF generation is only supported by Chromium. Calling `page.pdf()` on
/// Firefox or WebKit will result in an error.
///
/// See: <https://playwright.dev/docs/api/class-page#page-pdf>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct PdfOptions {
    /// If specified, the PDF will also be saved to this file path.
    pub path: Option<std::path::PathBuf>,
    /// Scale of the webpage rendering, between 0.1 and 2 (default 1).
    pub scale: Option<f64>,
    /// Whether to display header and footer (default false).
    pub display_header_footer: Option<bool>,
    /// HTML template for the print header. Should be valid HTML.
    pub header_template: Option<String>,
    /// HTML template for the print footer.
    pub footer_template: Option<String>,
    /// Whether to print background graphics (default false).
    pub print_background: Option<bool>,
    /// Paper orientation — `true` for landscape (default false).
    pub landscape: Option<bool>,
    /// Paper ranges to print, e.g. `"1-5, 8"`. Defaults to empty string (all pages).
    pub page_ranges: Option<String>,
    /// Paper format, e.g. `"Letter"` or `"A4"`. Overrides `width`/`height`.
    pub format: Option<String>,
    /// Paper width in CSS units, e.g. `"8.5in"`. Overrides `format`.
    pub width: Option<String>,
    /// Paper height in CSS units, e.g. `"11in"`. Overrides `format`.
    pub height: Option<String>,
    /// Whether or not to prefer page size as defined by CSS.
    pub prefer_css_page_size: Option<bool>,
    /// Paper margins, defaulting to none.
    pub margin: Option<PdfMargin>,
}

impl PdfOptions {
    /// Creates a new builder for PdfOptions
    pub fn builder() -> PdfOptionsBuilder {
        PdfOptionsBuilder::default()
    }
}

/// Builder for PdfOptions
#[derive(Debug, Clone, Default)]
pub struct PdfOptionsBuilder {
    path: Option<std::path::PathBuf>,
    scale: Option<f64>,
    display_header_footer: Option<bool>,
    header_template: Option<String>,
    footer_template: Option<String>,
    print_background: Option<bool>,
    landscape: Option<bool>,
    page_ranges: Option<String>,
    format: Option<String>,
    width: Option<String>,
    height: Option<String>,
    prefer_css_page_size: Option<bool>,
    margin: Option<PdfMargin>,
}

impl PdfOptionsBuilder {
    /// Sets the file path for saving the PDF
    pub fn path(mut self, path: std::path::PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Sets the scale of the webpage rendering
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = Some(scale);
        self
    }

    /// Sets whether to display header and footer
    pub fn display_header_footer(mut self, display: bool) -> Self {
        self.display_header_footer = Some(display);
        self
    }

    /// Sets the HTML template for the print header
    pub fn header_template(mut self, template: impl Into<String>) -> Self {
        self.header_template = Some(template.into());
        self
    }

    /// Sets the HTML template for the print footer
    pub fn footer_template(mut self, template: impl Into<String>) -> Self {
        self.footer_template = Some(template.into());
        self
    }

    /// Sets whether to print background graphics
    pub fn print_background(mut self, print: bool) -> Self {
        self.print_background = Some(print);
        self
    }

    /// Sets whether to use landscape orientation
    pub fn landscape(mut self, landscape: bool) -> Self {
        self.landscape = Some(landscape);
        self
    }

    /// Sets the page ranges to print
    pub fn page_ranges(mut self, ranges: impl Into<String>) -> Self {
        self.page_ranges = Some(ranges.into());
        self
    }

    /// Sets the paper format (e.g., `"Letter"`, `"A4"`)
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Sets the paper width
    pub fn width(mut self, width: impl Into<String>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the paper height
    pub fn height(mut self, height: impl Into<String>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Sets whether to prefer page size as defined by CSS
    pub fn prefer_css_page_size(mut self, prefer: bool) -> Self {
        self.prefer_css_page_size = Some(prefer);
        self
    }

    /// Sets the paper margins
    pub fn margin(mut self, margin: PdfMargin) -> Self {
        self.margin = Some(margin);
        self
    }

    /// Builds the PdfOptions
    pub fn build(self) -> PdfOptions {
        PdfOptions {
            path: self.path,
            scale: self.scale,
            display_header_footer: self.display_header_footer,
            header_template: self.header_template,
            footer_template: self.footer_template,
            print_background: self.print_background,
            landscape: self.landscape,
            page_ranges: self.page_ranges,
            format: self.format,
            width: self.width,
            height: self.height,
            prefer_css_page_size: self.prefer_css_page_size,
            margin: self.margin,
        }
    }
}
