// observability example — wire `tracing_subscriber` to see playwright-rs spans.
//
// Distinct from the `trace_on_failure` example: that one uses Playwright's
// own `Tracing` class (which writes a trace.zip openable at
// https://trace.playwright.dev). This one uses the Rust `tracing` crate
// ecosystem — the same instrumentation tonic / sqlx / reqwest emit, so
// playwright-rs operations show up alongside the rest of an app's spans.
//
// Run:
//     cargo run --package playwright-rs --example observability
//
// Tweak the env filter to widen or narrow the output:
//     RUST_LOG="playwright_rs=debug" cargo run --package playwright-rs --example observability
//     RUST_LOG="playwright_rs::protocol::page=debug,playwright_rs=info" cargo run ...
//
// Top-level operations (goto, click, screenshot, evaluate, browser.close)
// emit at `info`. Everything else is `debug`. Sensitive payloads (input
// values, eval expressions, request/response bodies) are deliberately
// excluded from span fields.

use anyhow::Result;
use playwright_rs::Playwright;
use tracing::Instrument;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Default to playwright_rs=info so the example is informative without
    // being noisy. Override with RUST_LOG to widen.
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("playwright_rs=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // A user span wrapping a logical workflow. Spans inside this block —
    // including async work spawned by playwright-rs internals — nest under
    // "checkout_flow" thanks to spawn-task propagation.
    async {
        page.goto("data:text/html,<button id='b'>Pay</button>", None)
            .await?;

        let button = page.locator("#b").await;
        button.click(None).await?;

        let _shot = page.screenshot(None).await?;

        Ok::<_, anyhow::Error>(())
    }
    .instrument(tracing::info_span!("checkout_flow", cart_id = 42))
    .await?;

    browser.close().await?;
    Ok(())
}
