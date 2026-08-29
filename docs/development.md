# Developing playwright-rust

Building, testing, and debugging the crate itself. For *using* the crate,
start at the [README](../README.md) and [docs.rs](https://docs.rs/playwright-rs).

## Prerequisites

- Rust 1.88+
- [cargo-nextest](https://nexte.st/): `cargo install cargo-nextest`

No system Node.js is needed for the normal build: the build script downloads
the pinned Playwright driver together with its own Node runtime. (With
`PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD` set, driver resolution falls back to
`PLAYWRIGHT_DRIVER_PATH` or an npm-installed playwright, which do need one.)

## Building from source

```bash
git clone https://github.com/padamson/playwright-rust.git
cd playwright-rust

# Install pre-commit hooks
pip install pre-commit
pre-commit install

cargo build
```

The build script downloads the Playwright driver into the build directory
(`$OUT_DIR/playwright-driver`); set `PLAYWRIGHT_DRIVER_CACHE_DIR` to relocate
it to a stable path that survives `cargo clean`, which is what CI does.

## Installing browsers

After building, install browsers with the in-repo example:

```bash
cargo run --package playwright-rs --example install-browsers -- chromium firefox webkit
```

Pass `--with-deps` on Linux CI to also install the system libraries the
browsers need. CI handles browser installation automatically; see
[`.github/workflows/test.yml`](../.github/workflows/test.yml).

## Running tests

```bash
cargo nextest run                                     # all tests
cargo nextest run -p playwright-rs --lib              # unit tests only (~2s, no browsers)
cargo nextest run -p playwright-rs -E 'test(locator)' # pattern match
cargo test --doc --workspace                          # doc-tests (compile-checked)
```

## Running examples

See [examples/](../crates/playwright/examples/) for usage examples.

```bash
cargo run --package playwright-rs --example basic
```

## Debugging test failures

When `?` propagates an `Error` out of a test, you see the message but no Rust
source location. Use [`anyhow`](https://docs.rs/anyhow) for tests and run with
`RUST_BACKTRACE=1`:

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

Run as `RUST_BACKTRACE=1 cargo nextest run`. The backtrace points at the
failing `?`, and `.context("...")` adds breadcrumbs to the error chain. This
matches how playwright-java/dotnet rely on the test runner's stack trace
rather than baking source locations into the library.

To save a Playwright trace when a test fails, see
[`examples/trace_on_failure.rs`](../crates/playwright/examples/trace_on_failure.rs)
and the [tracing section on docs.rs](https://docs.rs/playwright-rs). Open the
resulting `trace.zip` at <https://trace.playwright.dev>.
