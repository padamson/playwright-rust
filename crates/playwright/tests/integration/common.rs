use playwright_rs::protocol::{Browser, BrowserContext, Page, Playwright};
use std::sync::Once;

static INIT: Once = Once::new();

pub fn init_tracing() {
    INIT.call_once(|| {
        // Default to 'error' to keep tests quiet unless RUST_LOG is set
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error"));

        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    });
}

/// Launch Playwright + Chromium browser + new page.
///
/// Initializes tracing and provides a ready-to-use (Playwright, Browser, Page) tuple.
/// Panics with descriptive messages if any step fails.
pub async fn setup() -> (Playwright, Browser, Page) {
    init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("setup: failed to launch Playwright — is the driver installed?");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("setup: failed to launch Chromium — are browsers installed?");
    let page = browser
        .new_page()
        .await
        .expect("setup: failed to create new page");
    (playwright, browser, page)
}

/// Launch Playwright + Chromium browser + new context (without creating a page).
///
/// Use this when you need to configure the context before creating pages,
/// or when testing context-level features like `on_page` or `expect_page`.
pub async fn setup_context() -> (Playwright, Browser, BrowserContext) {
    init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("setup_context: failed to launch Playwright — is the driver installed?");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("setup_context: failed to launch Chromium — are browsers installed?");
    let context = browser
        .new_context()
        .await
        .expect("setup_context: failed to create new context");
    (playwright, browser, context)
}
