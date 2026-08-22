# Playwright for Rust

[![crates.io](https://img.shields.io/crates/v/playwright-rs.svg)](https://crates.io/crates/playwright-rs)
[![docs.rs](https://docs.rs/playwright-rs/badge.svg)](https://docs.rs/playwright-rs)
[![CI](https://github.com/padamson/playwright-rust/actions/workflows/test.yml/badge.svg)](https://github.com/padamson/playwright-rust/actions/workflows/test.yml)
[![License](https://img.shields.io/crates/l/playwright-rs)](LICENSE)
[![Playwright](https://img.shields.io/badge/Playwright-1.62.1-45ba4b)](https://playwright.dev)
[![skills.sh](https://skills.sh/b/padamson/playwright-rust)](https://skills.sh/padamson/playwright-rust)

> Rust language bindings for [Microsoft Playwright](https://playwright.dev) — the industry standard for cross-browser end-to-end testing.

**Status:** Pre-1.0, API stabilizing. See [coverage](#coverage) for the path to v1.0.

> This README describes the latest published release on crates.io. For changes
> on `main` that haven't been released yet (new features, breaking changes,
> bug fixes), see [`crates/playwright/CHANGELOG.md`](crates/playwright/CHANGELOG.md)
> under `[Unreleased]`.

## 🎯 Why playwright-rust?

Read our [WHY.md](WHY.md) to understand the vision, timing, and philosophy behind this project.

**TL;DR:** Rust is emerging as a serious web development language, with frameworks like Axum and Actix gaining traction. AI coding assistants are making Rust accessible to more developers. Test-Driven Development is experiencing a renaissance as the optimal way to work with AI agents.  **These trends are converging now, and they need production-quality E2E testing.** `playwright-rust` fills that gap by bringing Playwright's industry-leading browser automation to the Rust ecosystem.

## Roadmap and Goals

See [Development Roadmap](docs/roadmap.md) for plans and status of the development approach for `playwright-rust`.

**Goal:** Build this library to a production-quality state for broad adoption as `@playwright/rust` or `playwright-rs`. Provide official-quality Rust bindings for Microsoft Playwright, following the same architecture as [playwright-python](https://github.com/microsoft/playwright-python), [playwright-java](https://github.com/microsoft/playwright-java), and [playwright-dotnet](https://github.com/microsoft/playwright-dotnet).

## Quick Comparison: Python vs Rust

The API matches Playwright's cross-language conventions — if you know playwright-python, you know playwright-rust:

<table>
<tr><th>Python</th><th>Rust</th></tr>
<tr><td>

```python
from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page()
    page.goto("https://example.com")

    # Locator with auto-waiting
    heading = page.locator("h1")
    assert heading.text_content() == "Example Domain"

    # Response body access
    resp = page.goto("https://api.example.com/data")
    data = resp.json()

    browser.close()
```

</td><td>

```rust
use playwright_rs::Playwright;

let pw = Playwright::launch().await?;
let browser = pw.chromium().launch().await?;
let page = browser.new_page().await?;
page.goto("https://example.com", None).await?;

// Locator with auto-waiting
let heading = page.locator("h1").await;
assert_eq!(heading.text_content().await?, Some("Example Domain".into()));

// Response body access
let resp = page.goto("https://api.example.com/data", None).await?.unwrap();
let data: serde_json::Value = resp.json().await?;

browser.close().await?;
```

</td></tr>
</table>

## Coverage

**Full Python API parity + agent integration.** All Playwright Python
classes and methods are implemented, plus `Browser::bind()` /
`Browser::unbind()` (Playwright 1.59) for exposing a Rust-launched browser
to external clients like `@playwright/mcp`, the Playwright CLI, or
third-party agent tooling.

The remaining path to v1.0 is multi-month dogfooding, API polish, and
performance tuning rather than new surface area. See the
[v1.0 gap analysis](docs/implementation-plans/v1.0-gap-analysis.md) for
the detailed state of each class.

## How It Works

`playwright-rust` follows Microsoft's proven architecture for language bindings:

```
┌──────────────────────────────────────────────┐
│ playwright-rs (Rust API)                     │
│ - High-level, idiomatic Rust API             │
│ - Async/await with tokio                     │
│ - Type-safe bindings                         │
└─────────────────────┬────────────────────────┘
                      │ JSON-RPC over stdio
┌─────────────────────▼────────────────────────┐
│ Playwright Server (Node.js/TypeScript)       │
│ - Browser automation logic                   │
│ - Cross-browser protocol abstraction         │
│ - Maintained by Microsoft Playwright team    │
└─────────────────────┬────────────────────────┘
                      │ Native protocols
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
    Chromium      Firefox       WebKit
```

This means:
- ✅ **Full feature parity** with Playwright (JS/Python/Java/.NET)
- ✅ **Cross-browser support** (Chromium, Firefox, WebKit)
- ✅ **Automatic updates** when Playwright server updates
- ✅ **Minimal maintenance** - protocols handled by Microsoft's server
- ✅ **Production-tested** architecture used by millions

### API Design Philosophy

Following Playwright's cross-language consistency:

1. **Match Playwright API exactly** - Same method names, same semantics
2. **Idiomatic Rust** - Use Result<T>, async/await, builder patterns where appropriate
3. **Type safety** - Leverage Rust's type system for compile-time safety
4. **Auto-waiting** - Built-in smart waits like other Playwright implementations
5. **Testing-first** - Designed for reliable end-to-end testing

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
playwright-rs = "0.16"  # Auto-updates to latest 0.16.x
tokio = { version = "1", features = ["full"] }
```

The default-on `macros` feature re-exports the
[`locator!()`](https://docs.rs/playwright-rs-macros) compile-time
selector macro. The default-on `ring` feature selects the crypto backend for
driver downloads (and for rustls WebSocket connections); use `aws-lc` instead
when AWS-LC is required:

```toml
playwright-rs = { version = "0.16", default-features = false, features = [
    "aws-lc",
    "native-tls",
    "macros",
] }
```

Disabling default features requires selecting either `ring` or `aws-lc`.
If both are enabled through feature unification, the downloader uses AWS-LC;
disable default features as above to remove `ring` from the dependency graph.
Other opt-in features are `cli` (installer binary, see below) and
`screenshot-diff` (pixel-diff assertions). For programmatic trace-zip
inspection (CI bots, agent feedback loops), add
[`playwright-rs-trace`](https://docs.rs/playwright-rs-trace) as a
`[dev-dependencies]` entry.

See the [CHANGELOG](CHANGELOG.md) for version history and features.

### Browser Installation (Required)

Browsers must be installed before use. Install once, then run tests as many times as needed.

The library bundles a specific Playwright driver, and each driver release
expects matching browser builds. Install browsers through the crate itself
so the match is guaranteed and the browser version rides `Cargo.lock`:
copy
[`examples/install-browsers.rs`](crates/playwright/examples/install-browsers.rs)
into your project's `examples/` directory. (It needs `tokio` declared with
the `macros` and `rt-multi-thread` features.) Then:

```bash
# Install all browsers
cargo run --example install-browsers

# Or install specific browsers (in a workspace, add -p <your-package>)
cargo run --example install-browsers -- chromium firefox webkit
```

On Linux, required system libraries install automatically alongside the
browsers (the driver may invoke sudo for them). Pass `--with-deps` to
force that on other platforms; it is a no-op on macOS and needs elevation
on Windows.

**In CI/CD:** Add this to your GitHub Actions workflow:

```yaml
- name: Install Playwright browsers
  run: cargo run --example install-browsers -- chromium firefox webkit
```

When dependabot bumps `playwright-rs`, the crate, driver, and browsers now
move together with no workflow edit. Never hardcode a Playwright version
in a workflow `run:` step or a `package.json`: dependabot can't see the
former, and it bumps the latter on npm's cadence, not the crate's. The
driver ships its own Node.js runtime, so the workflow needs no setup-node
step. For setup scripts and Docker builds, call
[`install_browsers`](https://docs.rs/playwright-rs/latest/playwright_rs/fn.install_browsers.html)
directly.

**Outside a Cargo project:** the `cli` feature provides an installer binary
(`cargo install playwright-rs --features cli`, then `playwright-rs
install`). Note that `cargo install` builds its own copy of the crate,
which is a second version to keep in sync with your project's lockfile.
`npx playwright@<version> install` also works when `<version>` equals the
crate's bundled driver (`playwright_rs::PLAYWRIGHT_VERSION`, shown in the
badge above), but it goes stale silently when the crate bumps; in a Cargo
project, prefer the example approach.

**What happens if I don't install browsers?** You'll get an error message with install commands when trying to launch a browser.

## Development

### Prerequisites

- Rust 1.88+
- tokio async runtime

No system Node.js is needed for the normal build: the build script
downloads the pinned Playwright driver together with its own Node runtime.
(With `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD` set, driver resolution falls back
to `PLAYWRIGHT_DRIVER_PATH` or an npm-installed playwright, which do need
one.)

### Building from Source

```bash
# Clone repository
git clone https://github.com/YOUR_USERNAME/playwright-rust.git
cd playwright-rust

# Install pre-commit hooks
pip install pre-commit
pre-commit install

# Build
cargo build
```

### Installing Browsers

After building, install browsers with the in-repo example:

```bash
cargo run --package playwright-rs --example install-browsers -- chromium firefox webkit
```

The build script automatically downloads the Playwright driver into the
build directory (`$OUT_DIR/playwright-driver`); set
`PLAYWRIGHT_DRIVER_CACHE_DIR` to relocate it to a stable path that
survives `cargo clean`, which is what CI does. CI handles browser
installation automatically - see `.github/workflows/test.yml`.

**Platform Support:** ✅ Windows, macOS, Linux

**Known limitation**: WebKit `launch_persistent_context()` fails on native
Windows with "Initial load failed" — this is an upstream Playwright issue
([microsoft/playwright#36936](https://github.com/microsoft/playwright/issues/36936),
also tracked as playwright-rust [#39](https://github.com/padamson/playwright-rust/issues/39)).
Microsoft is building a `channel: "webkit-wsl"` replacement
([microsoft/playwright#37036](https://github.com/microsoft/playwright/issues/37036)).
Chromium and Firefox persistent contexts work on all platforms. Non-persistent
WebKit (`browser.new_context()`) works on Windows. Use WSL or macOS/Linux for
WebKit persistent contexts.

### Running Tests

This project uses [cargo-nextest](https://nexte.st/). Install once: `cargo install cargo-nextest`

```bash
cargo nextest run                                    # All tests
cargo nextest run -p playwright-rs --lib             # Unit tests only (~2s, no browsers)
cargo nextest run -p playwright-rs -E 'test(locator)' # Pattern match
cargo test --doc --workspace                         # Doc-tests (compile-checked)
```

### Running Examples

See [examples/](crates/playwright/examples/) for usage examples.

```bash
cargo run --package playwright-rs --example basic
```

## Testing & Debugging

**See the source line that failed.** When `?` propagates an `Error` out of
your test, you see the message but no Rust source location. Use
[`anyhow`](https://docs.rs/anyhow) for tests and run with `RUST_BACKTRACE=1`:

```rust,ignore
use anyhow::{Context, Result};

#[tokio::test]
async fn my_test() -> Result<()> {
    // ...
    let content = heading.text_content().await.context("read heading")?;
    // ...
    Ok(())
}
```

Run as `RUST_BACKTRACE=1 cargo nextest run` (or `cargo test`). The backtrace
points at the failing `?`, and `.context("...")` adds breadcrumbs to the
error chain. This matches how playwright-java/dotnet rely on the test
runner's stack trace rather than baking source locations into the library.

**Save a Playwright trace when a test fails.** Rust has no async `Drop`,
so trace cleanup is explicit. Capture the test result, then run cleanup
unconditionally and pass the trace path only on failure:

```rust,ignore
let result = run_test_body(&context).await;
let stop_opts = result.is_err().then(|| TracingStopOptions::default().path("trace.zip"));
let _ = tracing.stop(stop_opts).await;
let _ = browser.close().await;
result?;
```

See [`examples/trace_on_failure.rs`](crates/playwright/examples/trace_on_failure.rs)
for a runnable end-to-end example. Open the resulting `trace.zip` at
<https://trace.playwright.dev>.

## Using with Claude Code / AI agents

This repo ships an [Agent Skill](https://agentskills.io) so your agent
writes against this crate's actual API model instead of guessing from
generic Playwright knowledge. It covers adding the dependency and
installing browsers, the object model, the builder and `locator!()`
conventions, auto-wait semantics, and capturing a trace when something
fails.

```bash
npx skills add padamson/playwright-rust
```

Works with [Claude Code](https://claude.ai/code),
[Codex](https://openai.com/codex/), [Cursor](https://cursor.com), and any
other [compatible agent](https://agentskills.io/clients).

Claude Code can also install it as a plugin, which tracks this repo:

```bash
/plugin marketplace add padamson/playwright-rust
/plugin install playwright-rs@playwright-rust
```

Third-party marketplaces don't refresh on their own, so pick up a later
version with both commands, catalog first:

```bash
/plugin marketplace update playwright-rust
/plugin update playwright-rs@playwright-rust
```

The skill lives at
[`skills/playwright-rs-usage/`](skills/playwright-rs-usage/). It points back
at [docs.rs](https://docs.rs/playwright-rs) and the
[examples](crates/playwright/examples/) for the API surface itself, being a
thin "what to reach for, what to avoid" overlay rather than a duplicate of
the API reference. A build gate keeps it honest: its code compiles against
the real crate, and it cannot omit a cargo feature or browser engine the
crate exposes.

## Star History

[![Star History Chart](https://api.star-history.com/chart?repos=padamson/playwright-rust&type=date&legend=top-left&sealed_token=Op9aMPhRKMrLuxPN6Gm9tDlM5gQgFHjO9mtlrqq3qU6DWkO5nSao2tjUHuzC3m8RPGKZ0nVsg9Eo1l77UQ4E3t8XOIJfxFOlbWwI3EzojNG70cDCp2c28YVNRYSrl91Tnc-mvTs14Y5Aonm4ri8iI-CPCnPPHvYYNGJsOG8X-Ggt8TtYgQX_5VF_GENT)](https://www.star-history.com/?type=date&repos=padamson%2Fplaywright-rust)

## Contributing

This project aims for **production-quality** Rust bindings matching Playwright's standards. Contributions should:

- Follow Playwright API conventions
- Include comprehensive tests
- Maintain type safety
- Document public APIs with examples
- Pass CI checks (fmt, clippy, tests)

## License

Apache-2.0 (same as Microsoft Playwright)

## Acknowledgments

- **Microsoft Playwright Team** - For the amazing browser automation framework
- **playwright-python** - API design reference
