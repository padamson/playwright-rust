//! Workspace build-tooling binary. See `cargo xtask --help`.
//!
//! Subcommands:
//! - `regenerate-trace-fixture` — drives a real Chromium session via
//!   `playwright-rs` and writes the resulting `.trace.zip` into
//!   `crates/playwright-rs-trace/tests/fixtures/`, which downstream
//!   parser tests consume.
//! - `verify-agent-docs` — extracts ` ```rust,no_run ` code blocks
//!   from `docs/agent/CLAUDE_SNIPPET.md` and
//!   `.claude/skills/playwright-rs-usage/SKILL.md`, then runs
//!   `cargo check` against them so they can't silently drift from the
//!   crate API.
//! - `verify-site-snippets` — wraps each `crates/site/snippets/*.rs`
//!   fragment shown on the landing page in a binding prelude and runs
//!   `cargo check`, so the site can't advertise code that doesn't
//!   compile.
//! - `verify-driver-version` — checks that every source/CI reference to
//!   the bundled Playwright version (rustdoc/example install hints,
//!   workflow cache keys) matches `PLAYWRIGHT_VERSION` in
//!   `crates/playwright/build.rs`. The README is excluded on purpose.

use anyhow::{Context as _, Result, bail};
use axum::Router;
use axum::routing::get;
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
    /// Compile-check the `rust,no_run` blocks in the agent-integration
    /// docs (`docs/agent/CLAUDE_SNIPPET.md` and
    /// `.claude/skills/playwright-rs-usage/SKILL.md`) so they can't
    /// drift from the real `playwright-rs` API.
    VerifyAgentDocs,
    /// Compile-check the Rust snippets rendered on the landing page
    /// (`crates/site/snippets/*.rs`) so the site can't advertise code
    /// that doesn't compile against the real `playwright-rs` API.
    VerifySiteSnippets,
    /// Verify that every source/CI reference to the bundled Playwright
    /// version (rustdoc/example install hints, workflow cache keys)
    /// matches the single source of truth, `PLAYWRIGHT_VERSION` in
    /// `crates/playwright/build.rs`. The README is intentionally excluded:
    /// it tracks the latest crates.io release, not the in-tree version.
    VerifyDriverVersion,
}

#[tokio::main]
async fn main() -> Result<()> {
    match Cmd::parse() {
        Cmd::RegenerateTraceFixture { out } => regenerate_trace_fixture(&out).await,
        Cmd::VerifyAgentDocs => verify_agent_docs(),
        Cmd::VerifySiteSnippets => verify_site_snippets(),
        Cmd::VerifyDriverVersion => verify_driver_version(),
    }
}

async fn regenerate_trace_fixture(out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create parent dir {}", parent.display()))?;
    }

    // Local server so the navigation produces a `resource-snapshot`
    // in `trace.network` — `data:` URLs don't.
    let app = Router::new().route(
        "/",
        get(|| async { axum::response::Html(FIXTURE_PAGE_HTML) }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind fixture server")?;
    let addr = listener.local_addr().context("local_addr")?;
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let pw = Playwright::launch()
        .await
        .context("launch playwright server")?;
    let browser = pw.chromium().launch().await.context("launch chromium")?;
    let context = browser.new_context().await.context("new browser context")?;
    let tracing = context.tracing().await.context("get tracing handle")?;

    tracing
        .start(Some(
            TracingStartOptions::default()
                .name("fixture")
                .screenshots(true)
                .snapshots(true),
        ))
        .await
        .context("start tracing")?;

    let page = context.new_page().await.context("new page")?;

    page.goto(&format!("http://{addr}/"), None)
        .await
        .context("goto local fixture server")?;

    page.locator("#b")
        .click(None)
        .await
        .context("click button")?;

    // Brief pause so the console event is recorded before stop.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let out_str = out.to_string_lossy().into_owned();
    tracing
        .stop(Some(TracingStopOptions::default().path(out_str.clone())))
        .await
        .context("stop tracing")?;

    browser.close().await.context("close browser")?;
    server.abort();

    println!("wrote {}", out.display());
    Ok(())
}

const FIXTURE_PAGE_HTML: &str = r#"<!doctype html>
<html>
<body>
<button id="b" onclick='console.log("hi")'>X</button>
</body>
</html>"#;

/// Walk the agent-doc markdown files, extract every ` ```rust,no_run `
/// block, write them into a throwaway crate under
/// `target/agent-docs-verify/`, and run `cargo check` against it. If
/// the snippets drift from the real `playwright-rs` API, this fails.
fn verify_agent_docs() -> Result<()> {
    let workspace_root = workspace_root();
    let inputs = [
        workspace_root.join("docs/agent/CLAUDE_SNIPPET.md"),
        workspace_root.join(".claude/skills/playwright-rs-usage/SKILL.md"),
    ];

    let mut blocks: Vec<ExtractedBlock> = Vec::new();
    for path in &inputs {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        blocks.extend(extract_no_run_blocks(path, &content));
    }

    if blocks.is_empty() {
        println!("verify-agent-docs: no `rust,no_run` blocks found — nothing to check");
        return Ok(());
    }

    let check_dir = workspace_root.join("target/agent-docs-verify");
    std::fs::create_dir_all(check_dir.join("tests"))
        .with_context(|| format!("create {}", check_dir.display()))?;

    let playwright_path = workspace_root.join("crates/playwright");
    let cargo_toml = format!(
        r#"# Generated by `cargo xtask verify-agent-docs`. Do not edit.
[workspace]

[package]
name = "agent-docs-verify"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
anyhow = "1"
tokio = {{ version = "1", features = ["macros", "rt-multi-thread"] }}
playwright-rs = {{ path = {playwright_path:?} }}
"#,
        playwright_path = playwright_path.display().to_string(),
    );
    std::fs::write(check_dir.join("Cargo.toml"), cargo_toml).context("write Cargo.toml")?;

    let mut tests_rs = String::from(
        "// Generated by `cargo xtask verify-agent-docs`. Do not edit.\n\
         #![allow(unused_imports, unused_variables, dead_code)]\n\n",
    );
    for block in &blocks {
        let rel = block
            .source
            .strip_prefix(&workspace_root)
            .unwrap_or(&block.source);
        tests_rs.push_str(&format!(
            "// === {} (line {}) ===\n",
            rel.display(),
            block.start_line
        ));
        tests_rs.push_str(&block.code);
        tests_rs.push_str("\n\n");
    }
    std::fs::write(check_dir.join("tests/agent_docs.rs"), tests_rs)
        .context("write tests/agent_docs.rs")?;

    let status = std::process::Command::new(env!("CARGO"))
        .args(["check", "--tests", "--manifest-path"])
        .arg(check_dir.join("Cargo.toml"))
        .status()
        .context("invoke cargo check")?;

    if !status.success() {
        bail!(
            "verify-agent-docs: cargo check failed — agent-doc snippets have drifted from the playwright-rs API. \
             See output above; offending source lines are noted in `target/agent-docs-verify/tests/agent_docs.rs`."
        );
    }

    println!(
        "verify-agent-docs: {} block(s) compile cleanly against playwright-rs",
        blocks.len()
    );
    Ok(())
}

fn verify_site_snippets() -> Result<()> {
    let workspace_root = workspace_root();
    let snippets_dir = workspace_root.join("crates/site/snippets");

    let mut paths: Vec<PathBuf> = std::fs::read_dir(&snippets_dir)
        .with_context(|| format!("read {}", snippets_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    paths.sort();

    if paths.is_empty() {
        println!("verify-site-snippets: no .rs snippets found — nothing to check");
        return Ok(());
    }

    let check_dir = workspace_root.join("target/site-snippets-verify");
    std::fs::create_dir_all(check_dir.join("tests"))
        .with_context(|| format!("create {}", check_dir.display()))?;

    let playwright_path = workspace_root.join("crates/playwright");
    let cargo_toml = format!(
        r#"# Generated by `cargo xtask verify-site-snippets`. Do not edit.
[workspace]

[package]
name = "site-snippets-verify"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
tokio = {{ version = "1", features = ["macros", "rt-multi-thread"] }}
tracing-subscriber = "0.3"
playwright-rs = {{ path = {playwright_path:?} }}
"#,
        playwright_path = playwright_path.display().to_string(),
    );
    std::fs::write(check_dir.join("Cargo.toml"), cargo_toml).context("write Cargo.toml")?;

    // Each snippet is a fragment shown on the landing page, not a full
    // program. Wrap it in an async fn whose prelude binds everything the
    // fragments assume to be in scope (pw / page / url / context / cards).
    let mut tests_rs = String::from(
        "// Generated by `cargo xtask verify-site-snippets`. Do not edit.\n\
         #![allow(unused_imports, unused_variables, unused_mut, dead_code)]\n\
         use playwright_rs::protocol::{Playwright, ScreenshotOptions, Viewport};\n\
         use playwright_rs::expect;\n\n",
    );
    for path in &paths {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("snippet")
            .replace('-', "_");
        let code =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let rel = path.strip_prefix(&workspace_root).unwrap_or(path);
        tests_rs.push_str(&format!(
            "// === {} ===\n\
             async fn snippet_{stem}() -> std::result::Result<(), Box<dyn std::error::Error>> {{\n\
             \x20   let pw = Playwright::launch().await?;\n\
             \x20   let browser = pw.chromium().launch().await?;\n\
             \x20   let context = browser.new_context().await?;\n\
             \x20   let page = context.new_page().await?;\n\
             \x20   let url = \"http://localhost:8080\";\n\
             \x20   let cards: Vec<(&str, &str)> = Vec::new();\n\
             \x20   {{\n",
            rel.display(),
        ));
        for line in code.lines() {
            tests_rs.push_str("        ");
            tests_rs.push_str(line);
            tests_rs.push('\n');
        }
        tests_rs.push_str("    }\n    Ok(())\n}\n\n");
    }
    std::fs::write(check_dir.join("tests/snippets.rs"), tests_rs)
        .context("write tests/snippets.rs")?;

    let status = std::process::Command::new(env!("CARGO"))
        .args(["check", "--tests", "--manifest-path"])
        .arg(check_dir.join("Cargo.toml"))
        // Compile-only: never launches a browser, so skip the ~42 MB
        // driver download in playwright-rs's build script.
        .env("PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD", "1")
        .status()
        .context("invoke cargo check")?;

    if !status.success() {
        bail!(
            "verify-site-snippets: cargo check failed — a landing-page snippet has drifted from the \
             playwright-rs API. See output above; sources are noted in \
             `target/site-snippets-verify/tests/snippets.rs`."
        );
    }

    println!(
        "verify-site-snippets: {} snippet(s) compile cleanly against playwright-rs",
        paths.len()
    );
    Ok(())
}

struct ExtractedBlock {
    source: PathBuf,
    start_line: usize,
    code: String,
}

/// Parse fenced markdown code blocks tagged ` ```rust,no_run `. The
/// closing fence (``` ``` ```) must match the opening; no nesting.
fn extract_no_run_blocks(path: &Path, content: &str) -> Vec<ExtractedBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut start_line = 0usize;
    let mut buf = String::new();

    for (idx, line) in content.lines().enumerate() {
        let lineno = idx + 1;
        let trimmed = line.trim();
        if !in_block {
            // Match `rust,no_run` with optional trailing whitespace.
            if trimmed == "```rust,no_run" {
                in_block = true;
                start_line = lineno + 1;
                buf.clear();
            }
        } else if trimmed == "```" {
            blocks.push(ExtractedBlock {
                source: path.to_path_buf(),
                start_line,
                code: buf.clone(),
            });
            in_block = false;
        } else {
            buf.push_str(line);
            buf.push('\n');
        }
    }

    blocks
}

/// Parses `PLAYWRIGHT_VERSION` out of `crates/playwright/build.rs`, the single
/// source of truth for the bundled driver version.
fn read_driver_version(root: &Path) -> Result<String> {
    read_version_const(
        &root.join("crates/playwright/build.rs"),
        "const PLAYWRIGHT_VERSION: &str = \"",
    )
}

/// Parses `NODE_VERSION` out of the shared acquisition module, the single
/// source of truth for the Node runtime the driver is assembled with.
fn read_node_version(root: &Path) -> Result<String> {
    read_version_const(
        &root.join("crates/playwright/src/build_support/driver_urls.rs"),
        "const NODE_VERSION: &str = \"",
    )
}

fn read_version_const(path: &Path, marker: &str) -> Result<String> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let name = marker
        .trim_start_matches("const ")
        .split(':')
        .next()
        .unwrap_or(marker);
    let start = content
        .find(marker)
        .with_context(|| format!("{name} constant not found in {}", path.display()))?
        + marker.len();
    let end = content[start..]
        .find('"')
        .with_context(|| format!("unterminated {name} string literal"))?
        + start;
    Ok(content[start..end].to_string())
}

/// Collects every `MAJOR.MINOR.PATCH` token that immediately follows `prefix`
/// in `content` (e.g. `playwright@1.61.0` -> `1.61.0`).
fn versions_after(content: &str, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = content;
    while let Some(idx) = rest.find(prefix) {
        rest = &rest[idx + prefix.len()..];
        let ver: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if ver.contains('.') {
            out.push(ver);
        }
    }
    out
}

fn verify_driver_version() -> Result<()> {
    let root = workspace_root();
    let expected = read_driver_version(&root)?;

    // (file, prefixes whose trailing version must equal `expected`). The README
    // is deliberately absent: it mirrors the latest crates.io release, which may
    // lag the in-tree (unreleased) driver version until the next release. Same
    // reason for `hero.rs`'s `PLAYWRIGHT_RELEASED` — only its `PLAYWRIGHT_DEV`
    // (which tracks main HEAD) is anchored below.
    let targets: &[(&str, &[&str])] = &[
        ("crates/playwright/src/lib.rs", &["playwright@"]),
        (
            "crates/playwright/examples/connect_over_cdp.rs",
            &["playwright@"],
        ),
        (
            ".github/workflows/test.yml",
            &["pw-driver-", "playwright-browsers-"],
        ),
        (
            ".github/workflows/release.yml",
            &["pw-driver-", "playwright-browsers-"],
        ),
        (
            ".github/workflows/pages.yml",
            &["pw-driver-", "playwright-browsers-"],
        ),
        // Site dev-build driver badge + its e2e assertion, both tracking main
        // HEAD. Anchored so `PLAYWRIGHT_RELEASED` (the published-release driver,
        // which lags) is left out.
        (
            "crates/site/src/components/hero.rs",
            &["PLAYWRIGHT_DEV: &str = \""],
        ),
        (
            "crates/site-e2e/tests/landing_page.rs",
            &["alt='Playwright "],
        ),
    ];

    let mut drift = Vec::new();
    let mut checked = 0usize;
    for (rel, prefixes) in targets {
        let content =
            std::fs::read_to_string(root.join(rel)).with_context(|| format!("read {rel}"))?;
        for prefix in *prefixes {
            for found in versions_after(&content, prefix) {
                checked += 1;
                if found != expected {
                    drift.push(format!("  {rel}: `{prefix}{found}` (expected {expected})"));
                }
            }
        }
    }

    if !drift.is_empty() {
        bail!(
            "verify-driver-version: {} reference(s) disagree with build.rs \
             PLAYWRIGHT_VERSION = {expected}:\n{}\n\
             Bump them to {expected}. (The README is excluded on purpose — it \
             tracks the published crates.io release.)",
            drift.len(),
            drift.join("\n"),
        );
    }

    // The Node pin the driver is assembled with (ADR 0006). No cross-file
    // drift targets yet — it lives only in driver_urls.rs — but a rename or
    // malformed value must fail here loudly, because the weekly upstream
    // check greps the constant to probe the nodejs.org download URL.
    let node = read_node_version(&root)?;
    if node.split('.').count() != 3 || node.split('.').any(|p| p.parse::<u32>().is_err()) {
        bail!("verify-driver-version: NODE_VERSION = \"{node}\" is not a bare MAJOR.MINOR.PATCH");
    }

    println!(
        "verify-driver-version: {checked} reference(s) match build.rs PLAYWRIGHT_VERSION = {expected}; NODE_VERSION = {node}"
    );
    Ok(())
}

/// Resolve the workspace root by walking up from the xtask binary's
/// `CARGO_MANIFEST_DIR` (which Cargo sets at compile time for the
/// xtask crate to `crates/xtask`).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("xtask manifest dir has two parents")
        .to_path_buf()
}

#[cfg(test)]
mod driver_version_tests {
    use super::*;

    #[test]
    fn versions_after_extracts_each_occurrence() {
        let text = "npx playwright@1.61.0 install\nkey: os-pw-driver-1.61.0\n\
                    os-playwright-browsers-1.61.0-v2";
        assert_eq!(versions_after(text, "playwright@"), vec!["1.61.0"]);
        assert_eq!(versions_after(text, "pw-driver-"), vec!["1.61.0"]);
        assert_eq!(versions_after(text, "playwright-browsers-"), vec!["1.61.0"]);
    }

    #[test]
    fn versions_after_flags_a_stale_token() {
        let text = "playwright@1.60.0 and playwright@1.61.0";
        assert_eq!(
            versions_after(text, "playwright@"),
            vec!["1.60.0", "1.61.0"]
        );
    }

    #[test]
    fn versions_after_ignores_prefix_without_version() {
        assert!(versions_after("playwright@latest", "playwright@").is_empty());
    }

    #[test]
    fn build_rs_version_is_three_part_semver() {
        let v = read_driver_version(&workspace_root()).unwrap();
        assert_eq!(
            v.split('.').count(),
            3,
            "expected MAJOR.MINOR.PATCH, got {v}"
        );
    }
}
