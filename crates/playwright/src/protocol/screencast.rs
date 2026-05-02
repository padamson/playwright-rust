//! Live screencast frame streaming, optional disk recording, and
//! action / chapter / HTML overlays.
//!
//! Available on every [`Page`](crate::protocol::Page) via
//! [`screencast()`](crate::protocol::Page::screencast). Once started,
//! the Playwright server streams JPEG frames as they're rendered,
//! delivered to handlers registered with [`Screencast::on_frame`].
//! Optionally records to disk via the [`Artifact`](crate::protocol::Artifact)
//! save-on-stop pathway, and can overlay action labels, chapter cards,
//! or arbitrary HTML on the streamed frames.
//!
//! The action / chapter / HTML overlay surfaces are useful for "agent
//! receipts" — an LLM-driven flow can produce annotated video logs of
//! what it did alongside the action log.
//!
//! # Disk recording vs the Video class
//!
//! [`Video`](crate::protocol::Video) and [`Screencast`] cover
//! complementary lifecycles, both backed by the same underlying
//! `Artifact` save mechanism:
//!
//! - **`Video`** — automatic, captures the entire page session from
//!   open to close. Enabled with `BrowserContextOptions::record_video`.
//!   Use when you want a continuous recording over the whole session.
//! - **`Screencast::start({ path })`** — user-initiated, captures only
//!   during the start/stop window, saves to `path` on stop. Use when
//!   you want a recording that brackets a specific phase.
//!
//! # Example
//!
//! ```ignore
//! use playwright_rs::{Playwright, ScreencastStartOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let pw = Playwright::launch().await?;
//!     let browser = pw.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!     let screencast = page.screencast();
//!
//!     // Stream frames live
//!     screencast.on_frame(|frame| async move {
//!         println!("got {} byte frame", frame.data.len());
//!         Ok(())
//!     });
//!
//!     screencast.start(ScreencastStartOptions {
//!         path: Some(std::path::PathBuf::from("/tmp/run.webm")),
//!         ..Default::default()
//!     }).await?;
//!
//!     page.goto("https://example.com", None).await?;
//!     screencast.show_chapter(
//!         "Logged in",
//!         Default::default(),
//!     ).await?;
//!
//!     screencast.stop().await?; // saves /tmp/run.webm
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-page#page-screencast>

use crate::error::Result;
use crate::protocol::page::Page;
use std::path::PathBuf;

/// A single frame emitted while a screencast is active. Wire format is
/// JPEG; `data` holds the raw bytes ready to write to disk or pass to
/// an image decoder.
#[derive(Debug, Clone)]
pub struct ScreencastFrame {
    /// JPEG-encoded frame bytes.
    pub data: Vec<u8>,
}

/// Options for [`Screencast::start`].
#[derive(Debug, Default, Clone)]
pub struct ScreencastStartOptions {
    /// Output frame size. When `None`, Playwright uses the page's
    /// current viewport size.
    pub size: Option<ScreencastSize>,
    /// JPEG quality, `0..=100`. Server default is implementation-defined.
    pub quality: Option<i32>,
    /// When set, the screencast is also recorded to a file at this
    /// path. The file is written on [`Screencast::stop`]. The recording
    /// covers only the active start/stop window — for a continuous
    /// "always-on" recording over the whole page session, use
    /// `BrowserContextOptions::record_video` instead (the `Video`
    /// class).
    pub path: Option<PathBuf>,
}

/// Pixel dimensions for a screencast frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreencastSize {
    pub width: i32,
    pub height: i32,
}

/// Position for the action-label overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionPosition {
    TopLeft,
    Top,
    TopRight,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl ActionPosition {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ActionPosition::TopLeft => "top-left",
            ActionPosition::Top => "top",
            ActionPosition::TopRight => "top-right",
            ActionPosition::BottomLeft => "bottom-left",
            ActionPosition::Bottom => "bottom",
            ActionPosition::BottomRight => "bottom-right",
        }
    }
}

/// Options for [`Screencast::show_actions`].
#[derive(Debug, Default, Clone)]
pub struct ShowActionsOptions {
    /// How long each action label stays on screen (milliseconds).
    pub duration: Option<f64>,
    /// Where the label appears.
    pub position: Option<ActionPosition>,
    /// Label font size, pixels.
    pub font_size: Option<i32>,
}

/// Options for [`Screencast::show_chapter`].
#[derive(Debug, Default, Clone)]
pub struct ChapterOptions {
    /// Optional second line under the chapter title.
    pub description: Option<String>,
    /// How long the chapter card stays on screen (milliseconds).
    pub duration: Option<f64>,
}

/// Options for [`Screencast::show_overlay`].
#[derive(Debug, Default, Clone)]
pub struct ShowOverlayOptions {
    /// How long the overlay stays on screen (milliseconds).
    pub duration: Option<f64>,
}

/// Identifier for an active HTML overlay; pass to
/// [`Screencast::remove_overlay`] to dismiss the overlay before its
/// duration expires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayId(pub String);

/// Live frame-streaming entry point. Obtained from
/// [`Page::screencast`](crate::protocol::Page::screencast).
#[derive(Clone)]
pub struct Screencast {
    page: Page,
}

impl Screencast {
    pub(crate) fn new(page: Page) -> Self {
        Self { page }
    }

    /// Begin streaming. Frames arrive on handlers registered via
    /// [`on_frame`](Self::on_frame); register them before calling
    /// `start` so no frames are missed.
    ///
    /// If `options.path` is set, the screencast is also recorded to
    /// disk; the file is written when [`stop`](Self::stop) is called.
    pub async fn start(&self, options: ScreencastStartOptions) -> Result<()> {
        self.page.screencast_start(options).await
    }

    /// Stop the screencast. If `start` was called with a `path`, the
    /// recorded file is written to that path before this call returns.
    pub async fn stop(&self) -> Result<()> {
        self.page.screencast_stop().await
    }

    /// Register a handler for incoming frames. Multiple handlers may be
    /// registered; they fire in order for each frame.
    pub fn on_frame<F, Fut>(&self, handler: F)
    where
        F: Fn(ScreencastFrame) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.page.screencast_on_frame(handler);
    }

    /// Overlay action labels on the streamed frames as actions occur.
    /// Pair with [`hide_actions`](Self::hide_actions) to stop.
    pub async fn show_actions(&self, options: ShowActionsOptions) -> Result<()> {
        self.page.screencast_show_actions(options).await
    }

    /// Stop overlaying action labels. No-op if not currently shown.
    pub async fn hide_actions(&self) -> Result<()> {
        self.page.screencast_hide_actions().await
    }

    /// Show a chapter card with the given title (and optional
    /// description). Useful for splitting a session into named phases
    /// for an agent's video log.
    pub async fn show_chapter(&self, title: &str, options: ChapterOptions) -> Result<()> {
        self.page.screencast_chapter(title, options).await
    }

    /// Render arbitrary HTML as an overlay. Returns an [`OverlayId`]
    /// you can pass to [`remove_overlay`](Self::remove_overlay) to
    /// dismiss it early; otherwise it dismisses itself after
    /// `options.duration` (if set) or stays until removed.
    pub async fn show_overlay(&self, html: &str, options: ShowOverlayOptions) -> Result<OverlayId> {
        self.page.screencast_show_overlay(html, options).await
    }

    /// Remove an overlay previously created via
    /// [`show_overlay`](Self::show_overlay). Idempotent.
    pub async fn remove_overlay(&self, id: OverlayId) -> Result<()> {
        self.page.screencast_remove_overlay(id).await
    }

    /// Toggle visibility of all currently-shown overlays without
    /// removing them. Useful for hiding overlays during a section the
    /// agent considers "noise" and re-showing them later.
    pub async fn set_overlay_visible(&self, visible: bool) -> Result<()> {
        self.page.screencast_set_overlay_visible(visible).await
    }
}
