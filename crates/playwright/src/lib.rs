// playwright: High-level Rust bindings for Microsoft Playwright
//
// This crate provides the public API for browser automation using Playwright.
//
// # Example
//
// ```no_run
// use playwright::Playwright;
//
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Launch Playwright
//     let playwright = Playwright::launch().await?;
//
//     // Launch a browser
//     let browser = playwright.chromium().launch().await?;
//
//     // Create a page
//     let page = browser.new_page().await?;
//
//     // Page starts at about:blank
//     assert_eq!(page.url(), "about:blank");
//
//     // Cleanup
//     page.close().await?;
//     browser.close().await?;
//
//     Ok(())
// }
// ```

// Re-export core types
pub use playwright_core::error::{Error, Result};

// Re-export Playwright main entry point and browser API
pub use playwright_core::protocol::{
    Browser, BrowserContext, BrowserType, GotoOptions, Page, Playwright, Response, WaitUntil,
};

// Re-export API types
pub use playwright_core::api::LaunchOptions;
