//! playwright: High-level Rust bindings for Microsoft Playwright
//!
//! This crate provides the public API for browser automation using Playwright.
//!
//! # Examples
//!
//! ## Basic Navigation and Interaction
//!
//! ```ignore
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
//! ```ignore
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
//! ```ignore
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
//! ```ignore
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

// Internal modules (exposed for integration tests)
#[doc(hidden)]
pub mod server;

pub mod api;
mod assertions;
mod error;
pub mod protocol;

/// Playwright server version bundled with this crate.
///
/// This version determines which browser builds are compatible.
/// When installing browsers, use this version to ensure compatibility:
///
/// ```bash
/// npx playwright@1.56.1 install
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
pub use assertions::expect;

// Re-export Playwright main entry point and browser API
pub use protocol::{Browser, BrowserContext, BrowserType, Page, Playwright, Response};

// Re-export Locator and element APIs
pub use protocol::{ElementHandle, Locator};

// Re-export navigation and page options
pub use protocol::{GotoOptions, WaitUntil};

// Re-export action options
pub use protocol::{
    CheckOptions, ClickOptions, FillOptions, HoverOptions, PressOptions, SelectOptions,
};

// Re-export form and input types
pub use protocol::{FilePayload, SelectOption};

// Re-export screenshot types
pub use protocol::{ScreenshotClip, ScreenshotOptions, ScreenshotType};

// Re-export browser context options and storage state types
pub use protocol::{
    BrowserContextOptions, Cookie, Geolocation, LocalStorageItem, Origin, RecordHar, RecordVideo,
    StorageState, Viewport,
};

// Re-export routing types
pub use protocol::{FulfillOptions, Route};

// Re-export launch options
pub use api::LaunchOptions;
