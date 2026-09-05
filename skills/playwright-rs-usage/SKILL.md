---
name: playwright-rs-usage
description: Procedural reference for using playwright-rs in Rust browser-automation code — object model (Browser/Context/Page/Locator), the `locator!()` macro, builder pattern for options, auto-wait semantics, adding the crate and installing its browsers, and how to capture / inspect traces for failure diagnosis. Use when writing tests or scripts with playwright-rs as a dependency. Loaded automatically when the current repo has playwright-rs in its Cargo.toml.
license: Apache-2.0
metadata:
  version: "0.7.0"
---

# Using playwright-rs

Rust bindings for [Microsoft Playwright](https://playwright.dev). This
crate is a thin JSON-RPC client to the upstream Playwright server, so
the API mirrors playwright-python / java / .NET semantics. When
unsure about a method's behavior, the
[upstream Playwright docs](https://playwright.dev/docs/api) are the
authoritative reference.

**Drift discipline.** The canonical API tour lives in the crate-level
rustdoc at <https://docs.rs/playwright-rs> — that's compile-checked
against the actual code. This skill is the "what to reach for, what
to avoid" overlay: durable conventions, not a method-by-method
reference. Two build gates hold it to the crate: the Rust code block
below is compiled against the real API, and every cargo feature and
browser engine the crate exposes must be named somewhere in this file,
so a capability cannot ship here undocumented.

Neither gate checks that this prose is *accurate*. That residue is
real. The text is written in concepts (auto-wait, builder pattern)
rather than specific method names so it ages slowly, but when it and
the crate disagree, the crate wins: check
<https://docs.rs/playwright-rs> for the version you actually have.

## Before any of this works

Two things, and the second one is the one that surprises people.

**The crate.** `playwright-rs` in `[dependencies]`, or `[dev-dependencies]`
if it is test-only, alongside `tokio` with the `full` feature. Defaults are
`native-tls`, `macros` and `ring`: transport, the `locator!()` macro, and
the crypto backend respectively. The release after 0.17.0 adds a fourth default,
`route-service`, in-process serving through `route_service`.

Prefer leaving them on. `default-features = false` is only needed to swap
`ring` for `aws-lc`, and it drops every default, so the replacements have to
be listed back explicitly or `locator!()` disappears and no TLS transport
remains (add `"route-service"` to the list on the release after 0.17.0):

```toml
playwright-rs = { version = "0.17", default-features = false, features = [
    "aws-lc",
    "native-tls",
    "macros",
] }
```

Two capabilities are opt-in and off unless asked for. `screenshot-diff`
turns on pixel-diff screenshot assertions, so reach for it when the task
calls for comparing a rendering against a baseline rather than asserting on
the DOM. `cli` builds an installer binary for use outside a Cargo project;
inside one, prefer the example below, because `cargo install` compiles a
second copy of the crate that then has to be kept in sync with the
project's lockfile.

**The browsers, which are a separate install and are required.** A fresh
checkout that only adds the dependency will fail at launch. The crate
bundles one pinned Playwright driver and each driver expects matching
browser builds, so install them *through the crate* rather than through a
global `npx playwright install`. That way the browser version rides
`Cargo.lock` and cannot drift from the driver. Copy
[`examples/install-browsers.rs`](https://github.com/padamson/playwright-rust/blob/main/crates/playwright/examples/install-browsers.rs)
into the project's `examples/` and run it once:

```bash
cargo run --example install-browsers                        # all
cargo run --example install-browsers -- chromium firefox    # or a subset
```

The same line belongs in CI before the test step. Never pin a Playwright
version in a workflow or a `package.json`: dependabot cannot see the
former and bumps the latter on npm's cadence rather than the crate's, and
either way the driver and browsers stop matching. The driver ships its own
Node runtime, so no `setup-node` step is needed. In a setup script or
Dockerfile, call
[`install_browsers`](https://docs.rs/playwright-rs/latest/playwright_rs/fn.install_browsers.html)
directly instead.

On Linux the browsers also need system libraries: pass `--with-deps` to
the example (CI usually wants this; it runs the package manager under
sudo), or call `install_browsers_with_deps` instead. Without it only
browsers install, on every platform.

## Object model

```text
Playwright            start here — Playwright::launch().await?
  └── BrowserType     .chromium() / .firefox() / .webkit()
        └── Browser   .launch().await? → owns the browser process
              └── BrowserContext       isolated cookies / storage
                    └── Page            one tab
                          └── Locator   selector with auto-wait
```

`Locator` is the workhorse. Build with `page.locator(...)` or the
semantic `get_by_*` helpers (`get_by_role`, `get_by_text`, etc. —
see docs.rs for the full list) and chain action / assertion methods.

## Conventions to follow

- **`Result<T>` and `async/await` on `tokio`.** One error type:
  `playwright_rs::Error`. Use `?` to propagate.
- **Builders / setters for option-heavy methods.** `goto`, `click`,
  `screenshot`, `fill`, `tracing().start`, etc. take an `Options`
  struct. These are `#[non_exhaustive]` (so upstream option additions
  stay non-breaking) — struct literals won't compile. Construct with
  the type's `builder()` where it has one, otherwise chain setters off
  `Default`/`new()`: `GetByRoleOptions::default().name("OK").exact(true)`,
  `Cookie::new(name, value).domain("example.com")`. The exact method
  names live on docs.rs; don't memorize them.
- **Auto-wait + auto-retry.** Locator-based actions wait until the
  element is actionable; `expect()` assertions retry until they hold
  or time out. **Never insert `tokio::time::sleep` between an action
  and a check** — that's a smell. If a wait feels necessary, you
  probably want `expect(locator).to_be_visible().await` or similar.
- **`locator!()` macro for literal selectors.** Compile-time validation
  catches typos and structural errors. Fall back to `&str` only for
  selectors computed at runtime.
- **No reimplemented browser protocols.** This crate is intentionally
  thin over the Playwright server. Anything you can't do via Playwright
  itself, you can't do here.

## Minimal test skeleton

This block is compile-checked by `cargo xtask verify-agent-docs` —
if the API drifts, the verifier fails:

```rust,no_run
use anyhow::Result;
use playwright_rs::{Playwright, locator, expect};

#[tokio::test]
async fn login_flow() -> Result<()> {
    let pw = Playwright::launch().await?;
    let browser = pw.chromium().launch().await?;
    let context = browser.new_context().await?;
    let page = context.new_page().await?;

    page.goto("https://example.com/login", None).await?;
    page.locator(locator!("input[name='user']")).fill("alice", None).await?;
    page.locator(locator!("input[name='pass']")).fill("hunter2", None).await?;
    page.locator(locator!("text=Sign in")).click(None).await?;

    expect(page.locator(locator!(".welcome"))).to_be_visible().await?;

    browser.close().await?;
    Ok(())
}
```

## Configuring the test runner

Under `cargo nextest`, browser tests fail out of the box for reasons that
have nothing to do with the test. Add `.config/nextest.toml` to the
consuming project before writing many of them:

```toml
[profile.default]
# A browser and its driver take longer than nextest's 100ms default to be
# reaped after a test ends. Without this, passing tests are intermittently
# flagged "leaky" and fail the run.
leak-timeout = "1s"
# Launching a browser is slow, and Firefox and WebKit are slower than
# Chromium. Worst on Windows.
slow-timeout = { period = "30s", terminate-after = 2 }

[profile.ci]
leak-timeout = "1s"
slow-timeout = { period = "60s", terminate-after = 2 }
retries = 1
```

`leak-timeout` is the one that will otherwise cost an afternoon: the
symptom is an intermittent "leaky" failure on a test whose assertions all
passed, which reads like a bug in the test.

Raise `slow-timeout` further for tests that drive Firefox or WebKit
specifically, using a `[[profile.default.overrides]]` block with a `filter`.
Reach for `retries` only in CI, where a retried browser launch is cheaper
than a re-run; locally it hides flakes you want to see.

Teardown itself is already synchronous: dropping `Playwright` blocks until
the driver exits, closing every browser cleanly rather than signalling them
(which would truncate in-flight traces, videos and HARs). In async code
prefer `playwright.shutdown().await`, which does the same work without
blocking a runtime thread.

## Capabilities worth reaching for

Concept-level pointers; the exact options live on docs.rs.

- **Stable / redacted screenshots.** `ScreenshotOptions` carries
  `animations(Disabled)` for flake-free shots (freeze CSS animations
  before capture) and `mask`/`mask_color` to overpaint dynamic or
  sensitive elements. Reach for `animations(Disabled)` whenever a
  screenshot races an animation.
- **Context-level events.** Beyond per-page handlers, `BrowserContext`
  observes activity across *all* its pages — `on_download`,
  `on_page_load` / `on_page_close`,
  `on_frame_attached` / `_detached` / `_navigated` — and
  `Browser::on_context` fires for each new context. Use these for
  multi-tab fixtures instead of wiring every page individually.
- **HAR network capture.** `tracing().start_har(path, ..)` /
  `stop_har()` records all network traffic to a HAR — inspect it in
  browser devtools or replay it deterministically with
  `route_from_har`. A sibling to trace capture.
- **Typed page probes.** `page.evaluate::<R, T>(expr, arg)`
  deserializes the JS return value straight into any serde
  `Deserialize` type — define a struct for the shape and skip manual
  parsing (`None::<&()>` for the no-argument case). `evaluate_value`
  (returns `String`) is only for one-off scalar probes; if you catch
  yourself returning delimited strings from JS and splitting them in
  Rust, switch to `evaluate`. Runnable walkthroughs:
  `examples/evaluate_typed.rs` and `examples/canvas_pixels.rs` (canvas
  pixel assertions for wasm/canvas frontends).
- **Drags: `Locator::drag_to` covers everything, including canvas.**
  It drives the real `pointerdown` → capture → `pointermove` →
  `pointerup` chain (works against `setPointerCapture` UIs), and
  `DragToOptions::target_position` — an offset from the target's
  top-left — turns it into "drag to a coordinate": pass the containing
  canvas/stage as the target. Prefer it over held-button
  `Mouse::move_to` sequences, which hang on headless Linux. Don't
  hand-roll synthetic `PointerEvent` dispatch via evaluate.
- **External drag-and-drop.** `Locator::drop` simulates dragging files
  or data in from outside the page (upload zones), distinct from
  `drag_to`, which drags one element onto another within the page.
- **File System Access API flows.** `page.fake_file_system()` installs
  an opt-in fake of `showSaveFilePicker`/`showOpenFilePicker` (native
  dialogs no automation tool can drive): seed opens with
  `set_open_file`, assert saves with `last_saved_bytes()`, control
  permission state. Don't hand-roll an `add_init_script` picker shim.
- **Accessibility-tree assertions.** `expect_page(&page)
  .to_match_aria_snapshot(..)` (and the locator form) guard the page's
  ARIA structure as a regression check; `aria_snapshot` can emit
  `[box=..]` bounding boxes for visual/agent reasoning.

- **Waiting on arbitrary state: `wait_for_function`.** When there is no
  selector to wait on — a JS flag, a store, a counter —
  `page.wait_for_function("() => window.app?.ready", None)` polls a
  predicate and resolves to its value as a `JSHandle`. The locator form
  binds the matched element as the first argument
  (`locator.wait_for_function("el => el.dataset.state === 'done'", None)`)
  and returns `()`. Prefer this over `poll_until`-around-`evaluate`
  loops: the driver polls on `requestAnimationFrame` and enforces the
  timeout. `WaitForFunctionOptions::polling_interval` switches to timer
  polling (page-global form only).
- **Calling back into Rust: `evaluate_with_callback`.** Hands the
  expression a Rust closure as its argument — JS calls it, the
  arguments come to Rust, and the closure's return value resolves the
  JS promise. The expression may stash the function for later (event
  listeners); the binding lives until the page closes, so register once
  rather than in a loop. For a permanently installed `window.fn`, use
  `expose_function`/`expose_binding` instead.
- **Session save & replay.** `context.storage_state(None)` captures
  cookies and per-origin storage;
  `StorageStateOptions::default().credentials(true).indexed_db(true)`
  additionally captures WebAuthn passkeys and IndexedDB.
  `set_storage_state(state)` restores into any context — a **replace**,
  not a merge: the driver clears storage for every visited origin, and
  restoring a state without `credentials` disposes an installed virtual
  authenticator. The fast path for "log in once, reuse everywhere".
- **Opting out of auto-scroll.** Pointer actions scroll the target into
  view before acting; `ClickOptions::builder().scroll(Scroll::None)`
  makes the action fail instead — the way to assert something is
  *already* visible, or to avoid scroll side effects.
- **Serving the app from inside the test: `route_service` (the release
  after 0.17.0).** When the frontend under test is served by Rust (an axum `Router`, or a built
  wasm bundle in a directory via tower-http's `ServeDir`), hand the
  service to `page.route_service("https://app.example/**", service)` or
  the `BrowserContext` form and navigate to that origin. Every matching
  request is answered by the service in-process: no listener, no port,
  any origin including `https://`, works in sandboxes. Prefer it over
  binding an ephemeral port and spawning `axum::serve` for a test. Reach
  for a real listener instead when the app depends on streaming
  responses, server-sent events, WebSockets (those go through
  `route_web_socket`), or connection-level behavior, since route
  interception delivers whole bodies and has no connection. The
  `route_service` module rustdoc has the full contract and a wasm
  testing section.

## Debugging failures with traces

Rust has no async `Drop`, so trace cleanup is **explicit**. The
canonical pattern: capture the test result, run cleanup
unconditionally, pass the trace path only on failure.

See [`examples/trace_on_failure.rs`](https://github.com/padamson/playwright-rust/blob/main/crates/playwright/examples/trace_on_failure.rs)
for the runnable end-to-end version — it's compiled by
`cargo check --examples` so it can't silently rot.

To view a captured trace: `playwright show-trace trace.zip`. The
viewer is language-agnostic — same UI JS / Python users see. The
hosted version is at <https://trace.playwright.dev>.

## Programmatic trace inspection

For CI bots, agent feedback loops, or any code that wants to read what
happened in a trace without re-running the test, add the companion
crate as a `[dev-dependencies]` entry: `playwright-rs-trace = "0.1"`.

Use `TraceReader::actions()` to walk the reassembled action stream,
`TraceReader::network()` for HTTP traffic. The crate's `//!`
rustdoc on <https://docs.rs/playwright-rs-trace> has a runnable
example.

## Things that look like playwright-python but aren't quite

- **No `sync_playwright`.** Async only; everything awaits on `tokio`.
- **`Result`, not exceptions.** Use `?` to propagate.
- **No keyword arguments.** Options come through `Options` structs with
  `..Default::default()`, not `name=value` in method calls.
- **No async `Drop`.** Always close browsers / stop tracing explicitly
  in a cleanup block — don't rely on RAII for I/O.
- **Locators are values, not lazy proxies.** `page.locator(...)`
  returns a `Locator` you can `.clone()` and re-use cheaply.
- **Functions cannot ride inside evaluate args.** Python passes
  callables directly; here the callback is a dedicated parameter
  (`evaluate_with_callback`) since serialized data cannot carry a Rust
  closure.

## Common pitfalls

- **Manual sleeps before assertions.** Use `expect(..)` and let it
  auto-retry.
- **Closing the browser before `tracing.stop()`** — traces are written
  on stop, so order matters: stop tracing first, then close.
- **Hardcoding selectors as `&str`.** Switch to `locator!()` for any
  selector you write as a literal.

## References

- Full API: <https://docs.rs/playwright-rs>
- Runnable examples: <https://github.com/padamson/playwright-rust/tree/main/crates/playwright/examples>
- Upstream Playwright docs: <https://playwright.dev/docs/api>
- Trace format / parser: <https://docs.rs/playwright-rs-trace>
- `locator!()` macro: <https://docs.rs/playwright-rs-macros>
