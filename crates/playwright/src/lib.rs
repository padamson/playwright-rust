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
//     let playwright = Playwright::launch().await?;
//     println!("Playwright launched successfully!");
//     Ok(())
// }
// ```

// Re-export core types
pub use playwright_core::error::{Error, Result};

// Re-export Playwright main entry point
pub use playwright_core::protocol::Playwright;
