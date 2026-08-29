# Playwright for Rust

[![crates.io](https://img.shields.io/crates/v/playwright-rs.svg)](https://crates.io/crates/playwright-rs)
[![docs.rs](https://docs.rs/playwright-rs/badge.svg)](https://docs.rs/playwright-rs)
[![CI](https://github.com/padamson/playwright-rust/actions/workflows/test.yml/badge.svg)](https://github.com/padamson/playwright-rust/actions/workflows/test.yml)
[![License](https://img.shields.io/crates/l/playwright-rs)](LICENSE)
[![Playwright](https://img.shields.io/badge/Playwright-1.62.1-45ba4b)](https://playwright.dev)
[![skills.sh](https://skills.sh/b/padamson/playwright-rust)](https://skills.sh/padamson/playwright-rust)

> Rust language bindings for [Microsoft Playwright](https://playwright.dev) — the industry standard for cross-browser end-to-end testing.

**Status:** Pre-1.0, API stabilizing. See [coverage](#coverage) for the path to v1.0.

> This README describes the latest published release on crates.io. For
> unreleased changes on `main`, see the
> [CHANGELOG](crates/playwright/CHANGELOG.md) under `[Unreleased]`.

## Why playwright-rust?

Rust is emerging as a serious web development language, AI coding
assistants are making it accessible to more developers, and test-driven
development works well with AI agents. Those trends need production-quality
E2E testing, and `playwright-rust` fills that gap ([WHY.md](WHY.md) has the
fuller case). The goal is official-quality Rust bindings following the same
architecture as
[playwright-python](https://github.com/microsoft/playwright-python),
playwright-java, and playwright-dotnet; see the
[development roadmap](docs/roadmap.md) for plans and status.

## Quick comparison: Python vs Rust

If you know playwright-python, you know playwright-rust:

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

**Full Python API parity + agent integration.** All Playwright Python classes
and methods are implemented, plus `Browser::bind()` / `Browser::unbind()`
(Playwright 1.59) for exposing a Rust-launched browser to external clients
like `@playwright/mcp`, the Playwright CLI, or third-party agent tooling.
Chromium, Firefox, and WebKit are tested in CI on Linux, macOS, and Windows.

The remaining path to v1.0 is multi-month dogfooding, API polish, and
performance tuning rather than new surface area. See the
[v1.0 gap analysis](docs/implementation-plans/v1.0-gap-analysis.md) for the
detailed state of each class.

One known limitation: WebKit `launch_persistent_context()` fails on native
Windows (upstream issue
[microsoft/playwright#36936](https://github.com/microsoft/playwright/issues/36936),
tracked here as [#39](https://github.com/padamson/playwright-rust/issues/39)).
Non-persistent WebKit works on Windows; use WSL or macOS/Linux otherwise.

## How it works

`playwright-rust` follows Microsoft's architecture for language bindings:

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

The crate is a thin JSON-RPC client to the same server the official
bindings use, so feature parity and protocol maintenance come from upstream
rather than being reimplemented here. The API diverges only where Rust
idiom allows a better shape (`Result<T>`, builders for option-heavy
methods, compile-time-validated selectors).

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
playwright-rs = "0.17"  # Auto-updates to latest 0.17.x
tokio = { version = "1", features = ["full"] }
```

The default-on `macros` feature re-exports the
[`locator!()`](https://docs.rs/playwright-rs-macros) compile-time selector
macro. The default-on `ring` feature selects the crypto backend for driver
downloads (and for rustls WebSocket connections); use `aws-lc` instead when
AWS-LC is required:

```toml
playwright-rs = { version = "0.17", default-features = false, features = [
    "aws-lc",
    "native-tls",
    "macros",
] }
```

Disabling default features requires selecting either `ring` or `aws-lc`.
Other opt-in features are `cli` (installer binary, see below) and
`screenshot-diff` (pixel-diff assertions). For programmatic trace-zip
inspection, add [`playwright-rs-trace`](https://docs.rs/playwright-rs-trace)
as a `[dev-dependencies]` entry.

### Browser installation (required)

Browsers install once; the library bundles a specific Playwright driver,
and each driver release expects matching browser builds. Install through
the crate itself so the match is guaranteed and the browser version rides
`Cargo.lock`: copy
[`examples/install-browsers.rs`](crates/playwright/examples/install-browsers.rs)
into your project's `examples/` directory (it needs `tokio` with the
`macros` and `rt-multi-thread` features), then:

```bash
cargo run --example install-browsers                          # all browsers
cargo run --example install-browsers -- chromium firefox      # or a subset
```

Pass `--with-deps` to also install the system libraries the browsers need
(Linux CI typically wants this; it runs the package manager under sudo).
Without the flag only browsers install, on every platform, matching
`npx playwright install`.

**In CI**, the same command runs before the test step:

```yaml
- name: Install Playwright browsers
  run: cargo run --example install-browsers -- chromium firefox webkit
```

When dependabot bumps `playwright-rs`, the crate, driver, and browsers move
together with no workflow edit. Never hardcode a Playwright version in a
workflow `run:` step or a `package.json`: dependabot can't see the former
and bumps the latter on npm's cadence, not the crate's. The driver ships its
own Node.js runtime, so no setup-node step is needed. For setup scripts and
Docker builds, call
[`install_browsers`](https://docs.rs/playwright-rs/latest/playwright_rs/fn.install_browsers.html)
directly. Outside a Cargo project, the `cli` feature provides an installer
binary (`cargo install playwright-rs --features cli`, then
`playwright-rs install`).

## Using with Claude Code / AI agents

This repo ships an [Agent Skill](https://agentskills.io) so your agent writes
against this crate's actual API model instead of guessing from generic
Playwright knowledge. It covers adding the dependency and installing
browsers, the object model, the builder and `locator!()` conventions,
auto-wait semantics, and capturing a trace when something fails.

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

Third-party marketplaces don't refresh on their own; update with
`/plugin marketplace update playwright-rust` followed by
`/plugin update playwright-rs@playwright-rust`.

The skill lives at
[`skills/playwright-rs-usage/`](skills/playwright-rs-usage/) and points back
at [docs.rs](https://docs.rs/playwright-rs) and the
[examples](crates/playwright/examples/) for the API surface itself. A build
gate keeps it honest: its code compiles against the real crate, and it
cannot omit a cargo feature or browser engine the crate exposes.

## Development

Building from source, running the test suite, and debugging failures
(backtraces, saving a Playwright trace on failure) are covered in
[docs/development.md](docs/development.md).

[![Star History Chart](https://api.star-history.com/chart?repos=padamson/playwright-rust&type=date&legend=top-left&sealed_token=Op9aMPhRKMrLuxPN6Gm9tDlM5gQgFHjO9mtlrqq3qU6DWkO5nSao2tjUHuzC3m8RPGKZ0nVsg9Eo1l77UQ4E3t8XOIJfxFOlbWwI3EzojNG70cDCp2c28YVNRYSrl91Tnc-mvTs14Y5Aonm4ri8iI-CPCnPPHvYYNGJsOG8X-Ggt8TtYgQX_5VF_GENT)](https://www.star-history.com/?type=date&repos=padamson%2Fplaywright-rust)

## Contributing

Contributions should follow Playwright API conventions, include tests,
document public APIs with examples, and pass CI (fmt, clippy, tests).

## License

Apache-2.0 (same as Microsoft Playwright). Thanks to the Microsoft
Playwright team for the framework and the server this crate drives, and
to playwright-python for the API design reference.
