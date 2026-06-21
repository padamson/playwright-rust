//! playwright: High-level Rust bindings for Microsoft Playwright
//!
//! This crate provides the public API for browser automation using Playwright.
//!
//! # Quick tour
//!
//! ## Object model
//!
//! ```text
//! Playwright            start here — Playwright::launch().await?
//!   └── BrowserType     .chromium() / .firefox() / .webkit()
//!         └── Browser   .launch().await? → owns the browser process
//!               └── BrowserContext       isolated cookies / storage
//!                     └── Page            one tab
//!                           └── Locator   selector with auto-wait
//! ```
//!
//! [`Locator`] is the workhorse. Build one with [`Page::locator`] or one
//! of the semantic helpers (`get_by_role`, `get_by_text`, `get_by_label`,
//! `get_by_placeholder`, `get_by_test_id`, `get_by_alt_text`,
//! `get_by_title`), then call action / assertion methods on it — they
//! wait for the element to be actionable automatically.
//!
//! ## API conventions
//!
//! - **`Result<T>` everywhere.** One error type, [`Error`].
//! - **Builders for option-heavy methods.** `goto`, `click`, `screenshot`,
//!   `tracing.start`, etc. take an `Options` struct (`..Default::default()`).
//! - **Auto-wait by default.** Locator-based actions wait until the
//!   element is actionable; [`expect`] assertions auto-retry until the
//!   condition holds or times out. You almost never need a manual `sleep`.
//! - **Async/await on `tokio`.** Every method that does I/O is `async`.
//! - **Selectors validated at compile time.** Prefer the [`locator!`]
//!   macro (re-exported when the default `macros` feature is on) over
//!   raw `&str`: empty / malformed selectors fail at `cargo build`, not
//!   at runtime.
//!
//! ## Debugging failures
//!
//! Fastest path to "what happened" is tracing:
//!
//! 1. Wrap the test body with [`BrowserContext::tracing`] —
//!    `start({ snapshots, screenshots, sources })` → run → `stop({ path })`.
//! 2. Open the resulting `.trace.zip` with
//!    `playwright show-trace path/to/trace.zip` — the trace viewer is
//!    language-agnostic, the same UI JS / Python users see.
//! 3. To inspect a trace programmatically (CI bots, agent feedback
//!    loops), pull in [`playwright-rs-trace`](https://docs.rs/playwright-rs-trace)
//!    as a `[dev-dependencies]`.
//!
//! See [`examples/trace_on_failure.rs`](https://github.com/padamson/playwright-rust/blob/main/crates/playwright/examples/trace_on_failure.rs)
//! for the canonical Rust pattern (Rust has no async `Drop`, so cleanup
//! is explicit).
//!
//! ## Companion crates
//!
//! - [`playwright-rs-macros`](https://docs.rs/playwright-rs-macros) —
//!   compile-time-validated [`locator!`] macro. Default-on via the
//!   `macros` feature; surfaced here as `playwright_rs::locator!`.
//! - [`playwright-rs-trace`](https://docs.rs/playwright-rs-trace) —
//!   pure-Rust parser for `.trace.zip` files. Standalone; add to
//!   `[dev-dependencies]` for post-mortem analysis.
//!
//! ## For AI coding agents
//!
//! If you're using this crate with Claude Code or another coding agent,
//! see [`docs/agent/`](https://github.com/padamson/playwright-rust/tree/main/docs/agent)
//! for a copy-paste CLAUDE.md snippet and the
//! [`playwright-rs-usage` skill](https://github.com/padamson/playwright-rust/tree/main/.claude/skills/playwright-rs-usage)
//! you can drop into your project's `.claude/skills/`.
//!
//! # Examples
//!
//! ## Basic Navigation and Interaction
//!
//! ```no_run
//! use playwright_rs::{Playwright, SelectOption};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!
//!     // Navigate using data URL for self-contained test
//!     let _ = page.goto(
//!         "data:text/html,<html><body>\
//!             <h1 id='title'>Welcome</h1>\
//!             <button id='btn' onclick='this.textContent=\"Clicked\"'>Click me</button>\
//!         </body></html>",
//!         None
//!     ).await;
//!
//!     // Query elements with locators
//!     let heading = page.locator("#title").await;
//!     let text = heading.text_content().await?;
//!     assert_eq!(text, Some("Welcome".to_string()));
//!
//!     // Click button and verify result
//!     let button = page.locator("#btn").await;
//!     button.click(None).await?;
//!     let button_text = button.text_content().await?;
//!     assert_eq!(button_text, Some("Clicked".to_string()));
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Form Interaction
//!
//! ```no_run
//! use playwright_rs::{Playwright, SelectOption};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!
//!     // Create form with data URL
//!     let _ = page.goto(
//!         "data:text/html,<html><body>\
//!             <input type='text' id='name' />\
//!             <input type='checkbox' id='agree' />\
//!             <select id='country'>\
//!                 <option value='us'>USA</option>\
//!                 <option value='uk'>UK</option>\
//!                 <option value='ca'>Canada</option>\
//!             </select>\
//!         </body></html>",
//!         None
//!     ).await;
//!
//!     // Fill text input
//!     let name = page.locator("#name").await;
//!     name.fill("John Doe", None).await?;
//!     assert_eq!(name.input_value(None).await?, "John Doe");
//!
//!     // Check checkbox
//!     let checkbox = page.locator("#agree").await;
//!     checkbox.set_checked(true, None).await?;
//!     assert!(checkbox.is_checked().await?);
//!
//!     // Select option
//!     let select = page.locator("#country").await;
//!     select.select_option("uk", None).await?;
//!     assert_eq!(select.input_value(None).await?, "uk");
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Element Screenshots
//!
//! ```no_run
//! use playwright_rs::Playwright;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!
//!     // Create element to screenshot
//!     let _ = page.goto(
//!         "data:text/html,<html><body>\
//!             <div id='box' style='width:100px;height:100px;background:blue'></div>\
//!         </body></html>",
//!         None
//!     ).await;
//!
//!     // Take screenshot of specific element
//!     let element = page.locator("#box").await;
//!     let screenshot = element.screenshot(None).await?;
//!     assert!(!screenshot.is_empty());
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Assertions (expect API)
//!
//! ```no_run
//! use playwright_rs::{expect, Playwright};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!
//!     let _ = page.goto(
//!         "data:text/html,<html><body>\
//!             <button id='enabled'>Enabled</button>\
//!             <button id='disabled' disabled>Disabled</button>\
//!             <input type='checkbox' id='checked' checked />\
//!         </body></html>",
//!         None
//!     ).await;
//!
//!     // Assert button states with auto-retry
//!     let enabled_btn = page.locator("#enabled").await;
//!     expect(enabled_btn.clone()).to_be_enabled().await?;
//!
//!     let disabled_btn = page.locator("#disabled").await;
//!     expect(disabled_btn).to_be_disabled().await?;
//!
//!     // Assert checkbox state
//!     let checkbox = page.locator("#checked").await;
//!     expect(checkbox).to_be_checked().await?;
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Observability
//!
//! Every public async method on the user-facing types
//! (`Browser`, `BrowserContext`, `BrowserType`, `Page`, `Frame`, `Locator`,
//! `ElementHandle`, `Tracing`, `CDPSession`, `Debugger`, `Screencast`, plus
//! `Request`, `Response`, `Dialog`, `Download`, `Worker`, `FileChooser`)
//! is instrumented with the [`tracing`] crate. Wire up any
//! `tracing_subscriber` and you get spans for free, with cardinality-bounded
//! identifiers (`guid`, `selector`, `url`, `name`) and selected
//! completion-time fields (`status`, `bytes_len`, `count`, `version`).
//! Internal `tokio::spawn` sites propagate the caller's span via
//! `Instrument::in_current_span()` so user-registered handlers and event
//! fan-out tasks inherit the surrounding context.
//!
//! Levels: top-level user operations (`goto`, `click`, `fill`, `screenshot`,
//! `pdf`, `evaluate`, `tracing.start/stop`, `browser_type.launch`) are at
//! `info`; everything else is at `debug`. Sensitive payloads — input
//! values, eval expressions, request/response bodies — are deliberately
//! excluded from span fields.
//!
//! ```no_run
//! use tracing_subscriber::EnvFilter;
//!
//! tracing_subscriber::fmt()
//!     .with_env_filter(EnvFilter::new("playwright_rs=info"))
//!     .init();
//! ```

// Internal modules (exposed for integration tests)
#[doc(hidden)]
pub mod server;

pub mod api;
mod assertions;
mod error;
pub mod protocol;
mod tty_guard;

/// Playwright server version bundled with this crate.
///
/// This version determines which browser builds are compatible.
/// When installing browsers, use this version to ensure compatibility:
///
/// ```bash
/// npx playwright@1.61.0 install
/// ```
///
/// See: <https://playwright.dev/docs/browsers>
pub const PLAYWRIGHT_VERSION: &str = env!("PLAYWRIGHT_DRIVER_VERSION");

/// Default timeout in milliseconds for Playwright operations.
///
/// This matches Playwright's standard default across all language implementations (Python, Java, .NET, JS).
/// Required in Playwright 1.56.1+ when timeout parameter is not explicitly provided.
///
/// See: <https://playwright.dev/docs/test-timeouts>
pub const DEFAULT_TIMEOUT_MS: f64 = 30000.0;

// Re-export error types
pub use error::{Error, Result};

// Re-export assertions API
pub use assertions::{PageExpectation, expect, expect_page};

// Screenshot-diff types are gated on the optional feature. (`Animations` is
// always available via the protocol re-export below; it is shared with
// ScreenshotOptions.)
#[cfg(feature = "screenshot-diff")]
pub use assertions::{ScreenshotAssertionOptions, ScreenshotAssertionOptionsBuilder};

// Re-export Playwright main entry point and browser API
pub use protocol::{
    Browser, BrowserContext, BrowserType, FrameLocator, HeaderEntry, Page, Playwright, Response,
    Selectors,
};

// Re-export input device types
pub use protocol::{Keyboard, Mouse, Touchscreen};

// Re-export Request and related types
pub use protocol::{Request, ResourceTiming};

// Re-export Locator and element APIs
pub use protocol::{
    AriaRole, AriaSnapshotMode, AriaSnapshotOptions, BoundingBox, ElementHandle, FilterOptions,
    GetByRoleOptions, HighlightOptions, JSHandle, Locator,
};

// Re-export navigation and page options
pub use protocol::{GotoOptions, WaitUntil};

// Re-export action options
pub use protocol::{
    CheckOptions, ClickOptions, DragToOptions, DropOptions, DropOptionsBuilder, FillOptions,
    HoverOptions, PressOptions, PressSequentiallyOptions, SelectOptions, TapOptions,
    WaitForOptions, WaitForState,
};

// Re-export Position (needed for DragToOptions and other options)
pub use protocol::Position;

// Re-export form and input types
pub use protocol::{FilePayload, SelectOption};

// Re-export screenshot types
pub use protocol::{Animations, Caret, Scale, ScreenshotClip, ScreenshotOptions, ScreenshotType};

// Re-export screencast types
pub use protocol::{
    ActionCursor, ActionPosition, ChapterOptions, OverlayId, Screencast, ScreencastFrame,
    ScreencastSize, ScreencastStartOptions, ShowActionsOptions, ShowOverlayOptions,
};

// Re-export WebStorage (page.local_storage / session_storage)
pub use protocol::{WebStorage, WebStorageKind};

// Re-export new page method types
pub use protocol::{
    AddScriptTagOptions, ColorScheme, EmulateMediaOptions, ForcedColors, Media, PdfMargin,
    PdfOptions, ReducedMotion,
};

// Re-export browser context options and storage state types
pub use protocol::{
    BrowserContextOptions, Cookie, Geolocation, LocalStorageItem, Origin, RecordHar, RecordVideo,
    StorageState, Viewport,
};

// Re-export the tracing + HAR API. These were the last consumer-facing
// cohort still reachable only via `playwright_rs::protocol::*`; bringing
// them to the crate root matches every other type a consumer constructs
// (the `protocol::` paths still work).
pub use protocol::{
    HarContent, HarMode, StartHarOptions, Tracing, TracingStartOptions, TracingStopOptions,
};

// Re-export EventWaiter for use with expect_page() / expect_close()
pub use protocol::EventWaiter;

// Re-export EventValue for use with expect_event()
pub use protocol::EventValue;

// Re-export ConsoleMessage types
pub use protocol::{ConsoleMessage, ConsoleMessageLocation};

// Re-export device descriptor types
pub use protocol::{DeviceDescriptor, DeviceViewport};

// Re-export WebError
pub use protocol::{WebError, WebErrorLocation};

// Re-export WebSocketRoute
pub use protocol::{WebSocketRoute, WebSocketRouteCloseOptions};

// Re-export FileChooser
pub use protocol::FileChooser;

// Re-export Accessibility and Coverage types
pub use protocol::{
    Accessibility, AccessibilitySnapshotOptions, CSSCoverageEntry, Coverage, CoverageRange,
    JSCoverageEntry, JSCoverageRange, JSFunctionCoverage, StartCSSCoverageOptions,
    StartJSCoverageOptions,
};

// Re-export Clock types
pub use protocol::{Clock, ClockInstallOptions};

// Re-export Video
pub use protocol::Video;

// Re-export routing types
pub use protocol::{FetchOptions, FetchResponse, FulfillOptions, Route, UnrouteBehavior};

// Re-export APIRequest public API
pub use protocol::{APIRequest, APIRequestContext, APIRequestContextOptions, APIResponse};

// Re-export launch and connection options
pub use api::{ConnectOverCdpOptions, LaunchOptions};

// Re-export browser installation helpers
pub use server::driver::{install_browsers, install_browsers_with_deps};

// Re-export the `locator!` compile-time-validated selector macro from
// the companion `playwright-rs-macros` crate. Gated on the `macros`
// feature (default-on) so users without the proc-macro toolchain
// available can opt out.
#[cfg(feature = "macros")]
pub use playwright_rs_macros::locator;
