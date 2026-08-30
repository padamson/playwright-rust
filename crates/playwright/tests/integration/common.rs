use playwright_rs::protocol::{Browser, BrowserContext, GotoOptions, Page, Playwright};
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

/// First navigation after a deliberately failed one, retried past the
/// error-page-commit interrupt.
///
/// A failed navigation can leave Chromium committing a
/// `chrome-error://chromewebdata/` page *after* `goto` has already returned
/// `Err`, and that commit legitimately interrupts an immediately-following
/// `goto` (upstream behavior, not a crate bug). Two error shapes are
/// retryable within the window: that interrupt, and a per-attempt timeout —
/// the attempt cap exists so one slow attempt cannot eat the whole window,
/// so hitting it must buy another attempt, not a panic. (Both arrive as
/// `Error::ProtocolError`; the crate never constructs `NavigationTimeout`.)
/// Anything else panics immediately with the context, attempt count, and
/// elapsed time, so a real goto regression cannot hide inside the window.
/// Every retry is logged at warn, with a capped exponential backoff so a
/// wedged page produces dozens of attempts in the log, not hundreds.
pub async fn goto_recovering(page: &Page, url: &str, context: &str) {
    const RECOVERY_WINDOW: std::time::Duration = std::time::Duration::from_secs(10);
    const ATTEMPT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
    const BACKOFF_CAP: std::time::Duration = std::time::Duration::from_millis(250);

    let start = tokio::time::Instant::now();
    let mut attempts = 0u32;
    let mut backoff = std::time::Duration::from_millis(25);
    loop {
        attempts += 1;
        match page
            .goto(url, Some(GotoOptions::new().timeout(ATTEMPT_TIMEOUT)))
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "{context}: recovered in {attempts} attempt(s), {:?}",
                    start.elapsed()
                );
                return;
            }
            Err(e) => {
                let retryable = matches!(
                    &e,
                    playwright_rs::Error::ProtocolError(m)
                        if m.contains("interrupted by another navigation")
                            || (m.contains("Timeout") && m.contains("exceeded"))
                );
                assert!(
                    retryable,
                    "{context}: navigation failed for a non-retryable reason after \
                     {attempts} attempt(s) in {:?}: {e:?}",
                    start.elapsed()
                );
                assert!(
                    start.elapsed() < RECOVERY_WINDOW,
                    "{context}: page did not recover after {attempts} attempt(s) \
                     in {:?}; last error: {e:?}",
                    start.elapsed()
                );
                tracing::warn!("{context} attempt {attempts} failed; retrying: {e:?}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(BACKOFF_CAP);
            }
        }
    }
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
