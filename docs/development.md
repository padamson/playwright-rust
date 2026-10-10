# Developing playwright-rust

Building, testing, and debugging the crate itself. For *using* the crate,
start at the [README](../README.md) and [docs.rs](https://docs.rs/playwright-rs).

## Prerequisites

- Rust 1.88+
- [cargo-nextest](https://nexte.st/): `cargo install cargo-nextest`
- [asbuilt](https://github.com/padamson/asbuilt), which the pre-commit hook
  runs, at the release `asbuilt.toml` pins:
  `cargo install asbuilt --version <pinned> --locked`. Any other release
  refuses to run and prints the exact command.

No system Node.js is needed for the normal build: the build script downloads
the pinned Playwright driver together with its own Node runtime. (With
`PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD` set, driver resolution falls through to an
npm-installed playwright, which does need one.) At runtime,
`PLAYWRIGHT_DRIVER_PATH` points at a different driver directory,
`PLAYWRIGHT_NODE_EXE` swaps the runtime that runs the driver's `cli.js`, and
`PLAYWRIGHT_CLI_JS` swaps the script; the rustdoc on
`server::driver::get_driver_executable` has the full order.

## Building from source

```bash
git clone https://github.com/padamson/playwright-rust.git
cd playwright-rust

# Install the pre-commit hooks (prek runs the repo's .pre-commit-config.yaml)
cargo install prek
prek install --overwrite

cargo build
```

The build script assembles the Playwright driver into the user cache
(`~/.cache/playwright-rust/<version>/` on Linux, `~/Library/Caches/` on
macOS, `%LOCALAPPDATA%` on Windows), the same place `playwright-rs install`
and the runtime lookup use, so one download serves every target directory,
every cargo-mutants copy, and `cargo clean`. Deleting it is safe: the build
script watches the assembled files and reassembles on the next build. Set
`PLAYWRIGHT_DRIVER_CACHE_DIR` to put it somewhere else, and
`PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD=1` on jobs that compile but never launch a
browser. CI caches the user-cache directory alongside the browsers.

## Installing browsers

After building, install browsers with the in-repo example:

```bash
cargo run --package playwright-rs --example install-browsers -- chromium firefox webkit
```

Pass `--with-deps` on Linux CI to also install the system libraries the
browsers need. CI handles browser installation automatically; see
[`.github/workflows/test.yml`](../.github/workflows/test.yml).

## The architecture model

`docs/architecture/model.c4` is the [asbuilt](https://github.com/padamson/asbuilt)
survey of this repo, committed so that `asbuilt check` (the pre-commit hook and
the `Architecture model` CI job) can fail when the code moves and the model
does not. Beside it, `views.c4` is hand-written: the three views a newcomer
wants first (`context`, `server`, `protocol`), and `asbuilt.toml` names the
driver and the browsers as externals. What the tool records, its blind spots
and its commands are asbuilt's own docs, not repeated here.

A red check means the model is behind the code: re-survey and commit the
result.

```bash
asbuilt survey
```

The model is only meaningful against one asbuilt release, so `asbuilt.toml`
pins it (`asbuilt = "X.Y.Z"`): CI installs that release and any other refuses
to run. Moving to a newer release is one commit: bump the pin, install it,
re-survey, and commit the pin and the model together.

The CI job blocks merges only once it is listed in the repository ruleset's
required checks, which is a setting rather than a workflow file.

The rendered tree is published with every version of the landing site at
`https://playwright-rust.dev/<version>/architecture/` (`dev/` for main).
`pages.yml` runs `asbuilt docs` into `crates/site/public/architecture/` before
building, and a `site-e2e` gate checks the deployed tree against the committed
model.

## Splitting a large module

`page.rs` and `browser_context.rs` were each cut into a directory: `mod.rs`
keeps the struct, its field types and the re-exports; every `impl Page` or
`impl BrowserContext` block lives in a child file with the option types it
owns. What those cuts, and the handler-registry
unification before them, taught:

- Unifying duplicated code surfaces semantic drift, not only boilerplate:
  the registry pass found waiters drained LIFO against docs promising FIFO
  and dispatchers waking waiters and handlers in opposite orders. Budget
  review for behavior differences, not just for the move.
- Children import what they use explicitly; a `use super::*` glob hides each
  file's dependencies.
- rustdoc renders the parent module's own `impl` after every child's, so the
  block that should read first must also be a child, declared first.
- rustfmt sorts adjacent `mod` lines; a comment between two of them holds the
  rustdoc order.
- A sorted line-multiset diff of the old file against the new ones proves the
  cut moved every line and invented none.

The next candidates are `protocol/frame.rs` (about 2700 lines) and
`protocol/locator.rs` (about 2400), the two largest files left in the crate.

## Running tests

```bash
cargo nextest run                                     # all tests
cargo nextest run -p playwright-rs --lib              # unit tests only (~2s, no browsers)
cargo nextest run -p playwright-rs -E 'test(locator)' # pattern match
cargo test --doc --workspace                          # doc-tests (compile-checked)
```

## Mutation testing

`scripts/mutants.sh` runs `cargo mutants --in-diff` over the lines a commit
touched. `.cargo/mutants.toml` limits the mutated files to those with
unit-test coverage and the kill set to the browser-free test binaries;
[CLAUDE.md](../CLAUDE.md#mutation-testing) has the scope and the CI cadence.

The wrapper files (`protocol/frame.rs`, `protocol/page/navigation.rs`, ...)
are outside that scope because only the browser suite can kill their mutants. To
mutate one locally, bypass the config and have the browsers installed:

```bash
cargo mutants --no-config --list -f crates/playwright/src/protocol/frame.rs
```

`--list` prints the mutants and runs nothing (184 for `frame.rs`). Without
it, every mutant rebuilds the crate and runs the whole suite, browsers
included: minutes per mutant, hours per file. Narrow with `--re <fn>` and
set `--timeout`, and never put such a run in CI.

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
