//! Workspace build-tooling binary. See `cargo xtask --help`.
//!
//! Currently exposes `regenerate-trace-fixture` — drives a real
//! Chromium session via `playwright-rs` and writes the resulting
//! `.trace.zip` into `crates/playwright-rs-trace/tests/fixtures/`,
//! which downstream parser tests consume.

use anyhow::{Context as _, Result};
use clap::Parser;
use playwright_rs::Playwright;
use playwright_rs::protocol::{TracingStartOptions, TracingStopOptions};
use std::path::{Path, PathBuf};

/// `cargo xtask <subcommand>`
#[derive(Parser)]
#[command(name = "xtask", about = "Workspace build tooling for playwright-rust")]
enum Cmd {
    /// Regenerate the deterministic trace fixture used by
    /// playwright-rs-trace's parse tests.
    RegenerateTraceFixture {
        /// Output zip path (defaults to the fixture location).
        #[arg(
            long,
            default_value = "crates/playwright-rs-trace/tests/fixtures/basic.trace.zip"
        )]
        out: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    match Cmd::parse() {
        Cmd::RegenerateTraceFixture { out } => regenerate_trace_fixture(&out).await,
    }
}

/// Drive a deterministic playwright-rs session and write its trace zip
/// to `out`. Operations are chosen to exercise the event kinds the
/// slice-1 parser models: `context-options`, `before`/`input`/`log`/
/// `after` for an action, `console`, navigation, click. Resulting
/// fixture should stay in the tens-of-KB range.
async fn regenerate_trace_fixture(out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create parent dir {}", parent.display()))?;
    }

    let pw = Playwright::launch()
        .await
        .context("launch playwright server")?;
    let browser = pw.chromium().launch().await.context("launch chromium")?;
    let context = browser.new_context().await.context("new browser context")?;
    let tracing = context.tracing().await.context("get tracing handle")?;

    tracing
        .start(Some(TracingStartOptions {
            name: Some("fixture".into()),
            screenshots: Some(true),
            snapshots: Some(true),
            ..Default::default()
        }))
        .await
        .context("start tracing")?;

    let page = context.new_page().await.context("new page")?;

    // Deterministic content via data: URL — no network dependency, no
    // platform-specific paths.
    page.goto(
        "data:text/html,<button id='b' onclick='console.log(\"hi\")'>X</button>",
        None,
    )
    .await
    .context("goto data url")?;

    page.locator("#b")
        .await
        .click(None)
        .await
        .context("click button")?;

    // Brief pause so the console event is recorded before stop.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let out_str = out.to_string_lossy().into_owned();
    tracing
        .stop(Some(TracingStopOptions {
            path: Some(out_str.clone()),
        }))
        .await
        .context("stop tracing")?;

    browser.close().await.context("close browser")?;

    println!("wrote {}", out.display());
    Ok(())
}
