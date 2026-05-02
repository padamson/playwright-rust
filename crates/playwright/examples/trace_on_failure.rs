// trace_on_failure example - save a Playwright trace when a test fails,
// while ensuring browser/tracing cleanup always runs.
//
// Shows: anyhow + RUST_BACKTRACE=1 for failure-line diagnostics, plus the
// explicit try-finally pattern for trace-on-failure (no async Drop in Rust).
//
// Run:
//     cargo run --package playwright-rs --example trace_on_failure
//
// To see the failure path: edit the assertion below to fail, then re-run with
//     RUST_BACKTRACE=1 cargo run --package playwright-rs --example trace_on_failure
// On failure, `trace.zip` is written and can be opened at https://trace.playwright.dev.

use anyhow::{Context, Result, ensure};
use playwright_rs::Playwright;
use playwright_rs::protocol::{BrowserContext, TracingStartOptions, TracingStopOptions};

#[tokio::main]
async fn main() -> Result<()> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let context = browser.new_context().await?;
    let tracing = context.tracing().await?;

    tracing
        .start(Some(TracingStartOptions {
            name: Some("trace-on-failure".into()),
            screenshots: Some(true),
            snapshots: Some(true),
            ..Default::default()
        }))
        .await?;

    let result = run_test(&context).await;

    let trace_path = if result.is_err() {
        Some("trace.zip".to_string())
    } else {
        None
    };
    let _ = tracing
        .stop(Some(TracingStopOptions { path: trace_path }))
        .await;
    let _ = browser.close().await;

    result.context("test failed")
}

async fn run_test(context: &BrowserContext) -> Result<()> {
    let page = context.new_page().await?;
    page.goto("https://example.com", None).await?;

    let heading = page.locator("h1").await;
    let content = heading
        .text_content()
        .await
        .context("read h1 text content")?;

    ensure!(
        content.as_deref() == Some("Example Domain"),
        "heading mismatch: {content:?}"
    );
    Ok(())
}
