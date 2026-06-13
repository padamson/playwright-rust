use playwright_rs::protocol::{Browser, BrowserContext, Page, Playwright};
use std::path::PathBuf;
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

/// Resolve the Playwright `package/` directory via the crate's public driver
/// lookup. Returns `None` if the driver can't be found anywhere, so tests
/// that need to exec `node ... cli.js` can skip cleanly.
pub fn playwright_package_dir() -> Option<PathBuf> {
    let (_node, cli_js) = playwright_rs::server::driver::get_driver_executable().ok()?;
    cli_js.parent().map(PathBuf::from)
}

/// Poll `cond` until it returns `true` or `timeout` elapses; returns whether
/// it became true. Replaces "sleep a fixed N ms, then assert state changed"
/// patterns, which flake on loaded CI — this waits only as long as needed, up
/// to a generous bound, checking every 25ms.
pub async fn poll_until<F: FnMut() -> bool>(timeout: std::time::Duration, mut cond: F) -> bool {
    let start = std::time::Instant::now();
    loop {
        if cond() {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}
